//! =============================================================================
//! Modbus TCP Slave with start/stop, configurable port, and address map.
//! =============================================================================

use super::compiler::MemArea;
use super::memory::PlcMemory;
use super::modbus_map::{ModbusMap, ModbusTable, ResolvedBit, ResolvedReg, WriteProtectMode};
use parking_lot::Mutex;
use serde::Serialize;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_modbus::prelude::*;
use tokio_modbus::server::tcp::{accept_tcp_connection, Server};
use tokio_modbus::server::Service;
use tracing::{debug, error, info, warn};

pub const DEFAULT_MODBUS_PORT: u16 = 5020;
pub const DEFAULT_MODBUS_BIND: &str = "127.0.0.1";

#[derive(Debug, Clone, Serialize)]
pub struct ModbusStatus {
    pub enabled: bool,
    pub running: bool,
    pub port: u16,
    pub bind: String,
    pub write_enabled: bool,
    pub last_error: String,
}

struct PlcModbusService {
    memory: Arc<PlcMemory>,
    map: Arc<ModbusMap>,
}

impl Clone for PlcModbusService {
    fn clone(&self) -> Self {
        Self {
            memory: Arc::clone(&self.memory),
            map: Arc::clone(&self.map),
        }
    }
}

impl Service for PlcModbusService {
    type Request = Request<'static>;
    type Response = Response;
    type Exception = ExceptionCode;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Exception>> + Send>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let memory = Arc::clone(&self.memory);
        let map = Arc::clone(&self.map);
        Box::pin(async move {
            let started = std::time::Instant::now();
            let summary = describe_request(&req);
            let result = handle_request(&memory, &map, req);
            let elapsed_us = started.elapsed().as_micros() as u64;
            match &result {
                Ok(_) => {
                    debug!(target: "modbus", request = %summary, elapsed_us, "request served")
                }
                Err(e) => {
                    warn!(target: "modbus", request = %summary, exception = ?e, elapsed_us, "request rejected")
                }
            }
            result
        })
    }
}

// ─── Process-image accessors (area-aware) ────────────────────────────────────

fn get_plc_bit(memory: &PlcMemory, area: MemArea, index: u16) -> Result<bool, ExceptionCode> {
    match area {
        MemArea::Coil => memory
            .get_coil(index)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Discrete => memory
            .get_discrete(index)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::MemoryBit => memory
            .get_memory_bit(index)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Holding => memory
            .get_holding(index)
            .map(|v| v != 0)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::InputReg => memory
            .get_input_reg(index)
            .map(|v| v != 0)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::MemoryWord => memory
            .get_memory_word(index)
            .map(|v| v != 0)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
    }
}

fn set_plc_bit(
    memory: &PlcMemory,
    area: MemArea,
    index: u16,
    value: bool,
) -> Result<(), ExceptionCode> {
    match area {
        MemArea::Coil => memory
            .set_coil(index, value)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Discrete => memory
            .set_discrete(index, value)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::MemoryBit => memory
            .set_memory_bit(index, value)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Holding => memory
            .set_holding(index, if value { 1 } else { 0 })
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::InputReg => memory
            .set_input_reg(index, if value { 1 } else { 0 })
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::MemoryWord => memory
            .set_memory_word(index, if value { 1 } else { 0 })
            .map_err(|_| ExceptionCode::IllegalDataAddress),
    }
}

fn get_plc_word(memory: &PlcMemory, area: MemArea, index: u16) -> Result<u16, ExceptionCode> {
    match area {
        MemArea::Holding => memory
            .get_holding(index)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::InputReg => memory
            .get_input_reg(index)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::MemoryWord => memory
            .get_memory_word(index)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Coil => memory
            .get_coil(index)
            .map(|b| if b { 1 } else { 0 })
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Discrete => memory
            .get_discrete(index)
            .map(|b| if b { 1 } else { 0 })
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::MemoryBit => memory
            .get_memory_bit(index)
            .map(|b| if b { 1 } else { 0 })
            .map_err(|_| ExceptionCode::IllegalDataAddress),
    }
}

fn set_plc_word(
    memory: &PlcMemory,
    area: MemArea,
    index: u16,
    value: u16,
) -> Result<(), ExceptionCode> {
    match area {
        MemArea::Holding => memory
            .set_holding(index, value)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::InputReg => memory
            .set_input_reg(index, value)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::MemoryWord => memory
            .set_memory_word(index, value)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Coil => memory
            .set_coil(index, value != 0)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Discrete => memory
            .set_discrete(index, value != 0)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::MemoryBit => memory
            .set_memory_bit(index, value != 0)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
    }
}

fn get_word_bit(
    memory: &PlcMemory,
    area: MemArea,
    word_index: u16,
    bit: u8,
) -> Result<bool, ExceptionCode> {
    let word = get_plc_word(memory, area, word_index)?;
    Ok(((word >> bit) & 1) != 0)
}

fn set_word_bit(
    memory: &PlcMemory,
    area: MemArea,
    word_index: u16,
    bit: u8,
    value: bool,
) -> Result<(), ExceptionCode> {
    let word = get_plc_word(memory, area, word_index)?;
    let mask = 1u16 << bit;
    let next = if value { word | mask } else { word & !mask };
    set_plc_word(memory, area, word_index, next)
}

fn pack_bits(memory: &PlcMemory, area: MemArea, start: u16) -> Result<u16, ExceptionCode> {
    let mut word = 0u16;
    for i in 0u16..16 {
        let idx = start
            .checked_add(i)
            .ok_or(ExceptionCode::IllegalDataAddress)?;
        if get_plc_bit(memory, area, idx)? {
            word |= 1u16 << i;
        }
    }
    Ok(word)
}

fn unpack_bits(
    memory: &PlcMemory,
    area: MemArea,
    start: u16,
    value: u16,
) -> Result<(), ExceptionCode> {
    for i in 0u16..16 {
        let idx = start
            .checked_add(i)
            .ok_or(ExceptionCode::IllegalDataAddress)?;
        let bit = ((value >> i) & 1) != 0;
        set_plc_bit(memory, area, idx, bit)?;
    }
    Ok(())
}

// ─── Mapped Modbus table access ──────────────────────────────────────────────

fn read_bit_mapped(
    memory: &PlcMemory,
    map: &ModbusMap,
    table: ModbusTable,
    addr: u16,
) -> Result<bool, ExceptionCode> {
    let resolved = map
        .resolve_bit(table, addr)
        .ok_or(ExceptionCode::IllegalDataAddress)?;
    match resolved {
        ResolvedBit::Direct { area, index, .. } => get_plc_bit(memory, area, index),
        ResolvedBit::FromWord {
            area,
            word_index,
            bit,
            ..
        } => get_word_bit(memory, area, word_index, bit),
    }
}

/// Attempt a mapped bit write. Returns `Ok(true)` if applied, `Ok(false)` if
/// silently dropped (write-protected + SilentDrop), `Err` for Strict deny /
/// illegal address.
fn write_bit_mapped(
    memory: &PlcMemory,
    map: &ModbusMap,
    table: ModbusTable,
    addr: u16,
    value: bool,
) -> Result<bool, ExceptionCode> {
    let resolved = map
        .resolve_bit(table, addr)
        .ok_or(ExceptionCode::IllegalDataAddress)?;
    if resolved.write_protected() {
        return deny_write(map, "bit", addr);
    }
    match resolved {
        ResolvedBit::Direct { area, index, .. } => set_plc_bit(memory, area, index, value)?,
        ResolvedBit::FromWord {
            area,
            word_index,
            bit,
            ..
        } => set_word_bit(memory, area, word_index, bit, value)?,
    }
    Ok(true)
}

fn read_reg_mapped(
    memory: &PlcMemory,
    map: &ModbusMap,
    table: ModbusTable,
    addr: u16,
) -> Result<u16, ExceptionCode> {
    let resolved = map
        .resolve_reg(table, addr)
        .ok_or(ExceptionCode::IllegalDataAddress)?;
    match resolved {
        ResolvedReg::Direct { area, index, .. } => get_plc_word(memory, area, index),
        ResolvedReg::FromBits { area, start, .. } => pack_bits(memory, area, start),
    }
}

fn write_reg_mapped(
    memory: &PlcMemory,
    map: &ModbusMap,
    table: ModbusTable,
    addr: u16,
    value: u16,
) -> Result<bool, ExceptionCode> {
    let resolved = map
        .resolve_reg(table, addr)
        .ok_or(ExceptionCode::IllegalDataAddress)?;
    if resolved.write_protected() {
        return deny_write(map, "reg", addr);
    }
    match resolved {
        ResolvedReg::Direct { area, index, .. } => set_plc_word(memory, area, index, value)?,
        ResolvedReg::FromBits { area, start, .. } => unpack_bits(memory, area, start, value)?,
    }
    Ok(true)
}

/// Per-rule write protect handling. Global write-off is handled separately.
fn deny_write(map: &ModbusMap, kind: &str, addr: u16) -> Result<bool, ExceptionCode> {
    match map.write_protect_mode() {
        WriteProtectMode::Strict => {
            warn!(target: "modbus", kind, addr, "write blocked (rule write-protected, strict)");
            Err(ExceptionCode::IllegalDataAddress)
        }
        WriteProtectMode::SilentDrop => {
            warn!(target: "modbus", kind, addr, "write silently dropped (rule write-protected)");
            Ok(false)
        }
    }
}

/// Human-readable one-line summary of a Modbus request for logging.
fn describe_request(req: &Request<'static>) -> String {
    match req {
        Request::ReadCoils(a, q) => format!("ReadCoils(FC01) addr={a} qty={q}"),
        Request::ReadDiscreteInputs(a, q) => format!("ReadDiscreteInputs(FC02) addr={a} qty={q}"),
        Request::ReadHoldingRegisters(a, q) => {
            format!("ReadHoldingRegisters(FC03) addr={a} qty={q}")
        }
        Request::ReadInputRegisters(a, q) => format!("ReadInputRegisters(FC04) addr={a} qty={q}"),
        Request::WriteSingleCoil(a, v) => format!("WriteSingleCoil(FC05) addr={a} val={v}"),
        Request::WriteSingleRegister(a, v) => format!("WriteSingleRegister(FC06) addr={a} val={v}"),
        Request::WriteMultipleCoils(a, v) => {
            format!("WriteMultipleCoils(FC15) addr={a} qty={}", v.len())
        }
        Request::WriteMultipleRegisters(a, v) => {
            format!("WriteMultipleRegisters(FC16) addr={a} qty={}", v.len())
        }
        other => format!("{other:?}"),
    }
}

fn handle_request(
    memory: &PlcMemory,
    map: &ModbusMap,
    req: Request<'static>,
) -> Result<Response, ExceptionCode> {
    match req {
        Request::ReadCoils(addr, qty) => {
            if qty == 0 || qty > 2000 {
                return Err(ExceptionCode::IllegalDataValue);
            }
            let mut bits = Vec::with_capacity(qty as usize);
            for i in 0..qty {
                bits.push(read_bit_mapped(
                    memory,
                    map,
                    ModbusTable::Coil,
                    addr.saturating_add(i),
                )?);
            }
            Ok(Response::ReadCoils(bits))
        }
        Request::ReadDiscreteInputs(addr, qty) => {
            if qty == 0 || qty > 2000 {
                return Err(ExceptionCode::IllegalDataValue);
            }
            let mut bits = Vec::with_capacity(qty as usize);
            for i in 0..qty {
                bits.push(read_bit_mapped(
                    memory,
                    map,
                    ModbusTable::Discrete,
                    addr.saturating_add(i),
                )?);
            }
            Ok(Response::ReadDiscreteInputs(bits))
        }
        Request::ReadHoldingRegisters(addr, qty) => {
            if qty == 0 || qty > 125 {
                return Err(ExceptionCode::IllegalDataValue);
            }
            let mut regs = Vec::with_capacity(qty as usize);
            for i in 0..qty {
                regs.push(read_reg_mapped(
                    memory,
                    map,
                    ModbusTable::Holding,
                    addr.saturating_add(i),
                )?);
            }
            Ok(Response::ReadHoldingRegisters(regs))
        }
        Request::ReadInputRegisters(addr, qty) => {
            if qty == 0 || qty > 125 {
                return Err(ExceptionCode::IllegalDataValue);
            }
            let mut regs = Vec::with_capacity(qty as usize);
            for i in 0..qty {
                regs.push(read_reg_mapped(
                    memory,
                    map,
                    ModbusTable::InputReg,
                    addr.saturating_add(i),
                )?);
            }
            Ok(Response::ReadInputRegisters(regs))
        }
        Request::WriteSingleCoil(addr, value) => {
            if !memory.allow_modbus_write() {
                return Err(ExceptionCode::ServerDeviceFailure);
            }
            // SilentDrop may return Ok(false); response still acknowledges.
            let _applied = write_bit_mapped(memory, map, ModbusTable::Coil, addr, value)?;
            Ok(Response::WriteSingleCoil(addr, value))
        }
        Request::WriteSingleRegister(addr, value) => {
            if !memory.allow_modbus_write() {
                return Err(ExceptionCode::ServerDeviceFailure);
            }
            let _applied = write_reg_mapped(memory, map, ModbusTable::Holding, addr, value)?;
            Ok(Response::WriteSingleRegister(addr, value))
        }
        Request::WriteMultipleCoils(addr, values) => {
            if !memory.allow_modbus_write() {
                return Err(ExceptionCode::ServerDeviceFailure);
            }
            let qty = values.len() as u16;
            for (i, v) in values.iter().enumerate() {
                let _applied = write_bit_mapped(
                    memory,
                    map,
                    ModbusTable::Coil,
                    addr.saturating_add(i as u16),
                    *v,
                )?;
            }
            Ok(Response::WriteMultipleCoils(addr, qty))
        }
        Request::WriteMultipleRegisters(addr, values) => {
            if !memory.allow_modbus_write() {
                return Err(ExceptionCode::ServerDeviceFailure);
            }
            let qty = values.len() as u16;
            for (i, v) in values.iter().enumerate() {
                let _applied = write_reg_mapped(
                    memory,
                    map,
                    ModbusTable::Holding,
                    addr.saturating_add(i as u16),
                    *v,
                )?;
            }
            Ok(Response::WriteMultipleRegisters(addr, qty))
        }
        other => {
            warn!(?other, "unsupported Modbus function");
            Err(ExceptionCode::IllegalFunction)
        }
    }
}

/// Bind the listener, retrying briefly on `AddrInUse` (WSAEADDRINUSE / os error 10048).
///
/// On Windows a port held by a just-closed listener stays unbindable for ~1–3 s and
/// `SO_REUSEADDR` is intentionally not used (it would allow socket hijacking), so a
/// stop→start / port-change cycle can otherwise fail to rebind. Retrying with a short
/// backoff makes the restart deterministic without weakening socket security.
async fn bind_with_retry(
    addr: SocketAddr,
    port: u16,
    attempts: u32,
) -> std::io::Result<TcpListener> {
    let mut last: Option<std::io::Error> = None;
    for attempt in 1..=attempts {
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                if attempt > 1 {
                    info!(target: "modbus", %port, attempt, "bind succeeded after retry");
                }
                return Ok(listener);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                warn!(target: "modbus", %port, attempt, error = %e,
                    "port busy (os error 10048), retrying in 200ms");
                last = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| std::io::Error::new(std::io::ErrorKind::AddrInUse, "bind failed")))
}

/// Run the Modbus TCP serve loop until `shutdown` fires. Extracted so it can be
/// driven directly from an integration test over a real loopback socket.
async fn run_server(
    listener: TcpListener,
    memory: Arc<PlcMemory>,
    map: Arc<ModbusMap>,
    shutdown: oneshot::Receiver<()>,
) {
    let server = Server::new(listener);
    let on_connected = move |stream, socket_addr| {
        let mem = Arc::clone(&memory);
        let map = Arc::clone(&map);
        async move {
            info!(target: "modbus", %socket_addr, "client connected");
            let service = PlcModbusService { memory: mem, map };
            accept_tcp_connection(stream, socket_addr, move |_addr| Ok(Some(service.clone())))
        }
    };
    tokio::select! {
        res = server.serve(&on_connected, |err| {
            warn!(target: "modbus", error = %err, "client processing error / disconnect");
        }) => {
            if let Err(e) = res {
                error!(target: "modbus", error = %e, "server loop error");
            }
        }
        _ = shutdown => {
            info!(target: "modbus", "shutdown signal received");
        }
    }
}

/// Controllable Modbus TCP server (enable/disable, port change).
pub struct ModbusController {
    memory: Arc<PlcMemory>,
    map: Arc<ModbusMap>,
    port: AtomicU16,
    enabled: AtomicBool,
    running: AtomicBool,
    /// Generation counter: only the current server task may mutate `running`.
    epoch: AtomicU64,
    last_error: Mutex<String>,
    /// oneshot sender to stop current server
    stop_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl ModbusController {
    pub fn new(memory: Arc<PlcMemory>, map: Arc<ModbusMap>) -> Arc<Self> {
        Arc::new(Self {
            memory,
            map,
            port: AtomicU16::new(DEFAULT_MODBUS_PORT),
            enabled: AtomicBool::new(false),
            running: AtomicBool::new(false),
            epoch: AtomicU64::new(0),
            last_error: Mutex::new(String::new()),
            stop_tx: Mutex::new(None),
        })
    }

    pub fn map(&self) -> Arc<ModbusMap> {
        Arc::clone(&self.map)
    }

    pub fn status(&self) -> ModbusStatus {
        ModbusStatus {
            enabled: self.enabled.load(Ordering::SeqCst),
            running: self.running.load(Ordering::SeqCst),
            port: self.port.load(Ordering::SeqCst),
            bind: DEFAULT_MODBUS_BIND.into(),
            write_enabled: self.memory.allow_modbus_write(),
            last_error: self.last_error.lock().clone(),
        }
    }

    pub fn set_port(self: &Arc<Self>, port: u16) -> Result<(), String> {
        if port == 0 {
            return Err("port must be 1–65535".into());
        }

        let was_running = self.running.load(Ordering::SeqCst);
        if was_running {
            self.stop_internal();
        }
        self.port.store(port, Ordering::SeqCst);
        if was_running || self.enabled.load(Ordering::SeqCst) {
            self.start_internal()?;
        }
        Ok(())
    }

    pub fn start(self: &Arc<Self>) -> Result<ModbusStatus, String> {
        self.enabled.store(true, Ordering::SeqCst);
        self.start_internal()?;
        Ok(self.status())
    }

    pub fn stop(self: &Arc<Self>) -> ModbusStatus {
        self.enabled.store(false, Ordering::SeqCst);
        self.stop_internal();
        self.status()
    }

    fn stop_internal(&self) {
        // Invalidate the current task so its exit cannot clobber a later start.
        self.epoch.fetch_add(1, Ordering::SeqCst);
        if let Some(tx) = self.stop_tx.lock().take() {
            let _ = tx.send(());
        }
        self.running.store(false, Ordering::SeqCst);
        info!(target: "modbus", "Modbus TCP stop requested");
    }

    fn start_internal(self: &Arc<Self>) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        let port = self.port.load(Ordering::SeqCst);
        let (tx, rx) = oneshot::channel::<()>();
        *self.stop_tx.lock() = Some(tx);

        let my_epoch = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        let memory = Arc::clone(&self.memory);
        let map = Arc::clone(&self.map);
        let this = Arc::clone(self);

        tauri::async_runtime::spawn(async move {
            let addr: SocketAddr = match format!("{DEFAULT_MODBUS_BIND}:{port}").parse() {
                Ok(a) => a,
                Err(e) => {
                    error!(target: "modbus", error = %e, %port, "invalid bind address");
                    *this.last_error.lock() = e.to_string();
                    if this.epoch.load(Ordering::SeqCst) == my_epoch {
                        this.running.store(false, Ordering::SeqCst);
                    }
                    return;
                }
            };

            // Retry on WSAEADDRINUSE so a stop→start / port change rebinds cleanly.
            let listener = match bind_with_retry(addr, port, 20).await {
                Ok(l) => l,
                Err(e) => {
                    error!(target: "modbus", error = %e, %port,
                        "bind failed after retries (port busy or blocked)");
                    *this.last_error.lock() = format!("bind {port}: {e}");
                    if this.epoch.load(Ordering::SeqCst) == my_epoch {
                        this.running.store(false, Ordering::SeqCst);
                    }
                    return;
                }
            };

            if this.epoch.load(Ordering::SeqCst) == my_epoch {
                this.running.store(true, Ordering::SeqCst);
                *this.last_error.lock() = String::new();
            }
            info!(target: "modbus", %addr, "Modbus TCP slave listening");

            run_server(listener, memory, map, rx).await;

            if this.epoch.load(Ordering::SeqCst) == my_epoch {
                this.running.store(false, Ordering::SeqCst);
            }
            info!(target: "modbus", %port, "Modbus TCP slave stopped");
        });

        // Brief yield so an immediate bind error can be reported synchronously.
        std::thread::sleep(std::time::Duration::from_millis(50));
        if !self.running.load(Ordering::SeqCst) {
            let err = self.last_error.lock().clone();
            if !err.is_empty() {
                return Err(err);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn server_answers_read_holding_over_real_socket() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let memory = PlcMemory::new().into_arc();
        memory.set_holding(41, 0x1234).unwrap();
        let map = ModbusMap::new();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = oneshot::channel();
        let handle = tokio::spawn(run_server(listener, memory, map, rx));

        let mut stream = TcpStream::connect(addr).await.unwrap();
        // MBAP: txn=1 proto=0 len=6 unit=1 | PDU: FC=0x03 addr=41(0x0029) qty=1
        let req = [
            0x00u8, 0x01, 0x00, 0x00, 0x00, 0x06, 0x01, 0x03, 0x00, 0x29, 0x00, 0x01,
        ];
        stream.write_all(&req).await.unwrap();

        // Response = 7-byte MBAP + FC + byte-count + 2 data bytes = 11 bytes.
        let mut resp = [0u8; 11];
        stream.read_exact(&mut resp).await.unwrap();
        assert_eq!(resp[7], 0x03, "function code echoed");
        assert_eq!(resp[8], 0x02, "byte count = 2");
        assert_eq!(u16::from_be_bytes([resp[9], resp[10]]), 0x1234);

        let _ = tx.send(());
        let _ = handle.await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rebind_same_port_succeeds_after_stop() {
        // Bind an ephemeral port, learn its number, drop it, then rebind via the helper.
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = probe.local_addr().unwrap();
        drop(probe);
        let listener = bind_with_retry(addr, addr.port(), 20).await.unwrap();
        assert_eq!(listener.local_addr().unwrap().port(), addr.port());
    }

    #[test]
    fn internal_memory_reachable_only_via_explicit_map() {
        use crate::plc::modbus_map::{
            MappingType, ModbusMapEntry, ModbusMapSnapshot, WriteProtectMode,
        };

        let memory = PlcMemory::new().into_arc();
        memory.set_memory_bit(3, true).unwrap();
        memory.set_memory_word(2, 9).unwrap();

        // Explicit Direct rules expose M/MR (enterprise matrix).
        let map = ModbusMap::new();
        map.set_all(ModbusMapSnapshot {
            identity_fallback: false,
            write_protect_mode: WriteProtectMode::Strict,
            entries: vec![
                ModbusMapEntry {
                    id: "m".into(),
                    enabled: true,
                    mapping_type: MappingType::Direct,
                    symbol_name: String::new(),
                    plc_area: MemArea::MemoryBit,
                    plc_start: 3,
                    plc_bit_offset: 0,
                    modbus_table: ModbusTable::Coil,
                    modbus_start: 0,
                    length: 1,
                    is_write_protected: false,
                    comment: String::new(),
                },
                ModbusMapEntry {
                    id: "r".into(),
                    enabled: true,
                    mapping_type: MappingType::Direct,
                    symbol_name: String::new(),
                    plc_area: MemArea::MemoryWord,
                    plc_start: 2,
                    plc_bit_offset: 0,
                    modbus_table: ModbusTable::Holding,
                    modbus_start: 0,
                    length: 1,
                    is_write_protected: false,
                    comment: String::new(),
                },
            ],
        })
        .unwrap();

        assert!(read_bit_mapped(&memory, &map, ModbusTable::Coil, 0).unwrap());
        assert_eq!(
            read_reg_mapped(&memory, &map, ModbusTable::Holding, 0).unwrap(),
            9
        );
        memory.set_allow_modbus_write(true);
        assert!(write_bit_mapped(&memory, &map, ModbusTable::Coil, 0, false).unwrap());
        assert!(!memory.get_memory_bit(3).unwrap());
        assert!(write_reg_mapped(&memory, &map, ModbusTable::Holding, 0, 5).unwrap());
        assert_eq!(memory.get_memory_word(2).unwrap(), 5);

        // Identity fallback never invents M/MR exposure.
        map.set_all(ModbusMapSnapshot {
            identity_fallback: true,
            write_protect_mode: WriteProtectMode::Strict,
            entries: vec![],
        })
        .unwrap();
        // Coil 0 → Q0 identity, not M0.
        memory.set_coil(0, true).unwrap();
        memory.set_memory_bit(0, false).unwrap();
        assert!(read_bit_mapped(&memory, &map, ModbusTable::Coil, 0).unwrap());
        assert!(!memory.get_memory_bit(0).unwrap());
    }

    #[test]
    fn bit_to_register_packs_and_unpacks_memory_bits() {
        use crate::plc::modbus_map::{
            MappingType, ModbusMapEntry, ModbusMapSnapshot, WriteProtectMode,
        };

        let memory = PlcMemory::new().into_arc();
        for i in 0..16u16 {
            memory.set_memory_bit(i, i % 2 == 0).unwrap();
        }
        let map = ModbusMap::new();
        map.set_all(ModbusMapSnapshot {
            identity_fallback: false,
            write_protect_mode: WriteProtectMode::Strict,
            entries: vec![ModbusMapEntry {
                id: "b2r".into(),
                enabled: true,
                mapping_type: MappingType::BitToRegister,
                symbol_name: String::new(),
                plc_area: MemArea::MemoryBit,
                plc_start: 0,
                plc_bit_offset: 0,
                modbus_table: ModbusTable::Holding,
                modbus_start: 100,
                length: 1,
                is_write_protected: false,
                comment: String::new(),
            }],
        })
        .unwrap();

        // Even indices set → bits 0,2,4,...,14 → 0x5555
        assert_eq!(
            read_reg_mapped(&memory, &map, ModbusTable::Holding, 100).unwrap(),
            0x5555
        );

        memory.set_allow_modbus_write(true);
        write_reg_mapped(&memory, &map, ModbusTable::Holding, 100, 0x00FF).unwrap();
        for i in 0..16u16 {
            assert_eq!(
                memory.get_memory_bit(i).unwrap(),
                i < 8,
                "bit M{i} after unpack"
            );
        }
    }

    #[test]
    fn register_to_bit_extracts_status_bits() {
        use crate::plc::modbus_map::{
            MappingType, ModbusMapEntry, ModbusMapSnapshot, WriteProtectMode,
        };

        let memory = PlcMemory::new().into_arc();
        memory.set_holding(10, 0b1000_0000_0000_0001).unwrap(); // bits 0 and 15
        let map = ModbusMap::new();
        map.set_all(ModbusMapSnapshot {
            identity_fallback: false,
            write_protect_mode: WriteProtectMode::Strict,
            entries: vec![ModbusMapEntry {
                id: "r2b".into(),
                enabled: true,
                mapping_type: MappingType::RegisterToBit,
                symbol_name: String::new(),
                plc_area: MemArea::Holding,
                plc_start: 10,
                plc_bit_offset: 0,
                modbus_table: ModbusTable::Coil,
                modbus_start: 101,
                length: 16,
                is_write_protected: true,
                comment: String::new(),
            }],
        })
        .unwrap();

        assert!(read_bit_mapped(&memory, &map, ModbusTable::Coil, 101).unwrap());
        assert!(!read_bit_mapped(&memory, &map, ModbusTable::Coil, 102).unwrap());
        assert!(read_bit_mapped(&memory, &map, ModbusTable::Coil, 116).unwrap());

        // Write protected + Strict → exception, R10 unchanged.
        memory.set_allow_modbus_write(true);
        assert_eq!(
            write_bit_mapped(&memory, &map, ModbusTable::Coil, 101, false).unwrap_err(),
            ExceptionCode::IllegalDataAddress
        );
        assert_eq!(memory.get_holding(10).unwrap(), 0b1000_0000_0000_0001);
    }

    #[test]
    fn write_protect_silent_drop_acks_without_mutation() {
        use crate::plc::modbus_map::{
            MappingType, ModbusMapEntry, ModbusMapSnapshot, WriteProtectMode,
        };

        let memory = PlcMemory::new().into_arc();
        memory.set_holding(500, 42).unwrap();
        let map = ModbusMap::new();
        map.set_all(ModbusMapSnapshot {
            identity_fallback: false,
            write_protect_mode: WriteProtectMode::SilentDrop,
            entries: vec![ModbusMapEntry {
                id: "wp".into(),
                enabled: true,
                mapping_type: MappingType::Direct,
                symbol_name: String::new(),
                plc_area: MemArea::Holding,
                plc_start: 500,
                plc_bit_offset: 0,
                modbus_table: ModbusTable::Holding,
                modbus_start: 500,
                length: 1,
                is_write_protected: true,
                comment: String::new(),
            }],
        })
        .unwrap();

        memory.set_allow_modbus_write(true);
        // Handler returns success but value stays 42.
        handle_request(&memory, &map, Request::WriteSingleRegister(500, 99)).unwrap();
        assert_eq!(memory.get_holding(500).unwrap(), 42);
    }

    #[test]
    fn write_protect_strict_returns_illegal_data_address() {
        use crate::plc::modbus_map::{
            MappingType, ModbusMapEntry, ModbusMapSnapshot, WriteProtectMode,
        };

        let memory = PlcMemory::new().into_arc();
        memory.set_holding(500, 42).unwrap();
        let map = ModbusMap::new();
        map.set_all(ModbusMapSnapshot {
            identity_fallback: false,
            write_protect_mode: WriteProtectMode::Strict,
            entries: vec![ModbusMapEntry {
                id: "wp".into(),
                enabled: true,
                mapping_type: MappingType::Direct,
                symbol_name: String::new(),
                plc_area: MemArea::Holding,
                plc_start: 500,
                plc_bit_offset: 0,
                modbus_table: ModbusTable::Holding,
                modbus_start: 500,
                length: 1,
                is_write_protected: true,
                comment: String::new(),
            }],
        })
        .unwrap();

        memory.set_allow_modbus_write(true);
        let err = handle_request(&memory, &map, Request::WriteSingleRegister(500, 99)).unwrap_err();
        assert_eq!(err, ExceptionCode::IllegalDataAddress);
        assert_eq!(memory.get_holding(500).unwrap(), 42);
    }

    #[test]
    fn write_requests_require_explicit_enable() {
        let memory = PlcMemory::new().into_arc();
        let map = ModbusMap::new();

        let err = handle_request(&memory, &map, Request::WriteSingleCoil(0, true)).unwrap_err();
        assert_eq!(err, ExceptionCode::ServerDeviceFailure);
        assert!(!memory.get_coil(0).unwrap());

        memory.set_allow_modbus_write(true);
        handle_request(&memory, &map, Request::WriteSingleCoil(0, true)).unwrap();
        assert!(memory.get_coil(0).unwrap());
    }

    #[test]
    fn reads_resolve_all_tables_via_identity_map() {
        let memory = PlcMemory::new().into_arc();
        let map = ModbusMap::new();
        memory.set_coil(5, true).unwrap();
        memory.set_discrete(6, true).unwrap();
        memory.set_holding(7, 0xABCD).unwrap();
        memory.set_input_reg(2, 0x0042).unwrap();

        match handle_request(&memory, &map, Request::ReadCoils(5, 1)).unwrap() {
            Response::ReadCoils(bits) => assert_eq!(bits, vec![true]),
            other => panic!("unexpected {other:?}"),
        }
        match handle_request(&memory, &map, Request::ReadDiscreteInputs(6, 1)).unwrap() {
            Response::ReadDiscreteInputs(bits) => assert_eq!(bits, vec![true]),
            other => panic!("unexpected {other:?}"),
        }
        match handle_request(&memory, &map, Request::ReadHoldingRegisters(7, 1)).unwrap() {
            Response::ReadHoldingRegisters(regs) => assert_eq!(regs, vec![0xABCD]),
            other => panic!("unexpected {other:?}"),
        }
        match handle_request(&memory, &map, Request::ReadInputRegisters(2, 1)).unwrap() {
            Response::ReadInputRegisters(regs) => assert_eq!(regs, vec![0x0042]),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn quantity_limits_are_enforced() {
        let memory = PlcMemory::new().into_arc();
        let map = ModbusMap::new();

        // Zero quantity is illegal for every read.
        for req in [
            Request::ReadCoils(0, 0),
            Request::ReadDiscreteInputs(0, 0),
            Request::ReadHoldingRegisters(0, 0),
            Request::ReadInputRegisters(0, 0),
        ] {
            assert_eq!(
                handle_request(&memory, &map, req).unwrap_err(),
                ExceptionCode::IllegalDataValue
            );
        }

        // Over-limit quantities per Modbus spec (2000 bits / 125 regs).
        assert_eq!(
            handle_request(&memory, &map, Request::ReadCoils(0, 2001)).unwrap_err(),
            ExceptionCode::IllegalDataValue
        );
        assert_eq!(
            handle_request(&memory, &map, Request::ReadHoldingRegisters(0, 126)).unwrap_err(),
            ExceptionCode::IllegalDataValue
        );
    }

    #[test]
    fn unsupported_function_yields_illegal_function() {
        let memory = PlcMemory::new().into_arc();
        let map = ModbusMap::new();
        let err = handle_request(
            &memory,
            &map,
            Request::ReadWriteMultipleRegisters(0, 1, 0, std::borrow::Cow::Owned(vec![1])),
        )
        .unwrap_err();
        assert_eq!(err, ExceptionCode::IllegalFunction);
    }

    #[test]
    fn multiple_writes_require_enable_and_then_apply() {
        let memory = PlcMemory::new().into_arc();
        let map = ModbusMap::new();

        // Disabled: rejected, memory untouched.
        let err = handle_request(
            &memory,
            &map,
            Request::WriteMultipleRegisters(40, std::borrow::Cow::Owned(vec![11, 22, 33])),
        )
        .unwrap_err();
        assert_eq!(err, ExceptionCode::ServerDeviceFailure);
        assert_eq!(memory.get_holding(40).unwrap(), 0);

        // Enabled: applied.
        memory.set_allow_modbus_write(true);
        handle_request(
            &memory,
            &map,
            Request::WriteMultipleRegisters(40, std::borrow::Cow::Owned(vec![11, 22, 33])),
        )
        .unwrap();
        assert_eq!(memory.get_holding(40).unwrap(), 11);
        assert_eq!(memory.get_holding(41).unwrap(), 22);
        assert_eq!(memory.get_holding(42).unwrap(), 33);
    }

    /// Full network round-trip: real tokio-modbus client ↔ our TCP slave.
    #[tokio::test]
    async fn modbus_tcp_end_to_end_read_and_write_gate() {
        use tokio_modbus::client::tcp::connect;
        use tokio_modbus::client::{Reader, Writer};

        let memory = PlcMemory::new().into_arc();
        memory.set_holding(7, 4321).unwrap();
        memory.set_discrete(3, true).unwrap();
        let map = ModbusMap::new();

        // Ephemeral port avoids conflicts with the real service / other tests.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let service = PlcModbusService {
            memory: Arc::clone(&memory),
            map: Arc::clone(&map),
        };
        let server = tokio::spawn(async move {
            let server = Server::new(listener);
            let on_connected = move |stream, socket_addr| {
                let service = service.clone();
                async move {
                    accept_tcp_connection(stream, socket_addr, move |_addr| {
                        Ok(Some(service.clone()))
                    })
                }
            };
            let _ = server.serve(&on_connected, |_err| {}).await;
        });

        let mut ctx = connect(addr).await.expect("client connect");

        // Reads resolve through the identity map.
        let regs = ctx
            .read_holding_registers(7, 1)
            .await
            .expect("io")
            .expect("modbus");
        assert_eq!(regs, vec![4321]);
        let bits = ctx
            .read_discrete_inputs(3, 1)
            .await
            .expect("io")
            .expect("modbus");
        assert_eq!(bits, vec![true]);

        // Write disabled by default → Modbus exception, memory unchanged.
        let rejected = ctx.write_single_register(7, 999).await.expect("io");
        assert!(rejected.is_err(), "write must be rejected while disabled");
        assert_eq!(memory.get_holding(7).unwrap(), 4321);

        // Enable → write now applies over the wire.
        memory.set_allow_modbus_write(true);
        ctx.write_single_register(7, 999)
            .await
            .expect("io")
            .expect("modbus");
        assert_eq!(memory.get_holding(7).unwrap(), 999);

        ctx.write_single_coil(2, true)
            .await
            .expect("io")
            .expect("modbus");
        assert!(memory.get_coil(2).unwrap());

        let _ = ctx.disconnect().await;
        server.abort();
    }
}
