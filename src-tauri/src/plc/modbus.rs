//! =============================================================================
//! Modbus TCP Slave with start/stop, configurable port, and address map.
//! =============================================================================

use super::compiler::MemArea;
use super::memory::PlcMemory;
use super::modbus_map::{ModbusMap, ModbusTable};
use parking_lot::Mutex;
use serde::Serialize;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_modbus::prelude::*;
use tokio_modbus::server::tcp::{accept_tcp_connection, Server};
use tokio_modbus::server::Service;
use tracing::{error, info, warn};

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
        Box::pin(async move { handle_request(&memory, &map, req) })
    }
}

fn read_bit_mapped(
    memory: &PlcMemory,
    map: &ModbusMap,
    table: ModbusTable,
    addr: u16,
) -> Result<bool, ExceptionCode> {
    let (area, idx) = map
        .resolve_bit(table, addr)
        .ok_or(ExceptionCode::IllegalDataAddress)?;
    match area {
        MemArea::Coil => memory
            .get_coil(idx)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Discrete => memory
            .get_discrete(idx)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Holding => memory
            .get_holding(idx)
            .map(|v| v != 0)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::InputReg => memory
            .get_input_reg(idx)
            .map(|v| v != 0)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
    }
}

fn write_bit_mapped(
    memory: &PlcMemory,
    map: &ModbusMap,
    table: ModbusTable,
    addr: u16,
    value: bool,
) -> Result<(), ExceptionCode> {
    let (area, idx) = map
        .resolve_bit(table, addr)
        .ok_or(ExceptionCode::IllegalDataAddress)?;
    match area {
        MemArea::Coil => memory
            .set_coil(idx, value)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Discrete => memory
            .set_discrete(idx, value)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Holding => memory
            .set_holding(idx, if value { 1 } else { 0 })
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::InputReg => memory
            .set_input_reg(idx, if value { 1 } else { 0 })
            .map_err(|_| ExceptionCode::IllegalDataAddress),
    }
}

fn read_reg_mapped(
    memory: &PlcMemory,
    map: &ModbusMap,
    table: ModbusTable,
    addr: u16,
) -> Result<u16, ExceptionCode> {
    let (area, idx) = map
        .resolve_reg(table, addr)
        .ok_or(ExceptionCode::IllegalDataAddress)?;
    match area {
        MemArea::Holding => memory
            .get_holding(idx)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::InputReg => memory
            .get_input_reg(idx)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Coil => memory
            .get_coil(idx)
            .map(|b| if b { 1 } else { 0 })
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Discrete => memory
            .get_discrete(idx)
            .map(|b| if b { 1 } else { 0 })
            .map_err(|_| ExceptionCode::IllegalDataAddress),
    }
}

fn write_reg_mapped(
    memory: &PlcMemory,
    map: &ModbusMap,
    table: ModbusTable,
    addr: u16,
    value: u16,
) -> Result<(), ExceptionCode> {
    let (area, idx) = map
        .resolve_reg(table, addr)
        .ok_or(ExceptionCode::IllegalDataAddress)?;
    match area {
        MemArea::Holding => memory
            .set_holding(idx, value)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::InputReg => memory
            .set_input_reg(idx, value)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Coil => memory
            .set_coil(idx, value != 0)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
        MemArea::Discrete => memory
            .set_discrete(idx, value != 0)
            .map_err(|_| ExceptionCode::IllegalDataAddress),
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
            write_bit_mapped(memory, map, ModbusTable::Coil, addr, value)?;
            Ok(Response::WriteSingleCoil(addr, value))
        }
        Request::WriteSingleRegister(addr, value) => {
            if !memory.allow_modbus_write() {
                return Err(ExceptionCode::ServerDeviceFailure);
            }
            write_reg_mapped(memory, map, ModbusTable::Holding, addr, value)?;
            Ok(Response::WriteSingleRegister(addr, value))
        }
        Request::WriteMultipleCoils(addr, values) => {
            if !memory.allow_modbus_write() {
                return Err(ExceptionCode::ServerDeviceFailure);
            }
            let qty = values.len() as u16;
            for (i, v) in values.iter().enumerate() {
                write_bit_mapped(
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
                write_reg_mapped(
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

/// Controllable Modbus TCP server (enable/disable, port change).
pub struct ModbusController {
    memory: Arc<PlcMemory>,
    map: Arc<ModbusMap>,
    port: AtomicU16,
    enabled: AtomicBool,
    running: AtomicBool,
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
        if let Some(tx) = self.stop_tx.lock().take() {
            let _ = tx.send(());
        }
        self.running.store(false, Ordering::SeqCst);
        info!("Modbus TCP stopped");
    }

    fn start_internal(self: &Arc<Self>) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        let port = self.port.load(Ordering::SeqCst);
        let (tx, rx) = oneshot::channel::<()>();
        *self.stop_tx.lock() = Some(tx);

        let memory = Arc::clone(&self.memory);
        let map = Arc::clone(&self.map);
        let this = Arc::clone(self);

        tauri::async_runtime::spawn(async move {
            let addr: SocketAddr = match format!("{DEFAULT_MODBUS_BIND}:{port}").parse() {
                Ok(a) => a,
                Err(e) => {
                    *this.last_error.lock() = e.to_string();
                    this.running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let listener = match TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    error!(error = %e, %port, "Modbus bind failed");
                    *this.last_error.lock() = format!("bind {port}: {e}");
                    this.running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            this.running.store(true, Ordering::SeqCst);
            *this.last_error.lock() = String::new();
            info!(%addr, "Modbus TCP slave listening");

            let server = Server::new(listener);
            let on_connected = move |stream, socket_addr| {
                let mem = Arc::clone(&memory);
                let map = Arc::clone(&map);
                async move {
                    info!(%socket_addr, "Modbus client connected");
                    let service = PlcModbusService { memory: mem, map };
                    accept_tcp_connection(stream, socket_addr, move |_addr| {
                        Ok(Some(service.clone()))
                    })
                }
            };

            tokio::select! {
                res = server.serve(&on_connected, |err| {
                    error!(error = %err, "Modbus connection error");
                }) => {
                    if let Err(e) = res {
                        error!(error = %e, "Modbus server error");
                        *this.last_error.lock() = e.to_string();
                    }
                }

                _ = rx => {
                    info!("Modbus server abort signal received");
                }
            }
            this.running.store(false, Ordering::SeqCst);
        });

        // Brief yield for bind result
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
