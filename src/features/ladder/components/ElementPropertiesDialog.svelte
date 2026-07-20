<script lang="ts">
  /**
   * Dialog: assign I / Q / M / R / R1.x variables to ladder element.
   */
  import type { Address, CmpOp, LadderElement, MathOp } from "../../../shared/lib/types";
  import type { ElementType } from "../elements/_shared/types";
  import {
    ADDRESS_HELP_MD,
    formToAddress,
    formatAddress,
    parseVarString,
    type VarPrefix,
  } from "../lib/addressFormat";
  import {
    isBitElement,
    isCounterType,
    isTimerType,
  } from "../elements";

  interface Props {
    element: LadderElement;
    open: boolean;
    label?: string;
    onClose: () => void;
    onApply: (el: LadderElement, label: string) => void;
  }

  let { element, open, label = "", onClose, onApply }: Props = $props();

  let showHelp = $state(false);
  let quick = $state("");
  let parseError = $state("");
  let labelInput = $state("");

  // Primary address form (contacts/coils)
  let prefix = $state<VarPrefix>("I");
  let index = $state(0);
  let bit = $state(0);
  let useBit = $state(false);
  /** Switchable sub-type for bit elements (NO/NC/edge · coil//coil/SET/RESET). */
  let selectedType = $state<ElementType>("contact_no");

  // FB fields
  let presetMs = $state(1000);
  let timerIndex = $state(0);
  let presetC = $state(10);
  let counterIndex = $state(0);
  let mathOp = $state<MathOp>("add");
  let cmpOp = $state<CmpOp>("ge");

  // Multi-address for math/move/compare
  let addrA = $state("R0");
  let addrB = $state("R1");
  let addrDest = $state("R2");
  let addrDone = $state("Q1");
  let addrReset = $state("I3");

  const isBit = $derived(isBitElement(element.type));
  const isTimer = $derived(isTimerType(element.type));
  const isCounter = $derived(isCounterType(element.type));
  const isMath = $derived(element.type === "math");
  const isMove = $derived(element.type === "move");
  const isCmp = $derived(element.type === "compare");
  const isWire = $derived(element.type === "wire");
  const isContactFamily = $derived(
    (["contact_no", "contact_nc", "contact_rising", "contact_falling"] as string[]).includes(
      element.type
    )
  );
  const isCoilFamily = $derived(
    (["coil", "coil_negated", "coil_set", "coil_reset"] as string[]).includes(element.type)
  );

  $effect(() => {
    if (!open) return;
    showHelp = false;
    parseError = "";
    quick = "";
    labelInput = label;
    selectedType = element.type;
    syncFromElement(element);
  });

  function syncFromElement(el: LadderElement) {
    if ("address" in el) {
      const a = el.address;
      if (a.area === "discrete") prefix = "I";
      else if (a.area === "coil") prefix = "Q";
      else if (a.bit != null) prefix = "R";
      else prefix = "R";
      if (a.area === "holding" && a.bit != null) {
        // Prefer showing as R1.x; user can switch to M
        prefix = "R";
      }
      index = a.index;
      bit = a.bit ?? 0;
      useBit = a.bit != null;
      quick = formatAddress(a);
    }
    if (el.type === "ton" || el.type === "tof" || el.type === "rto") {
      presetMs = el.preset_ms;
      timerIndex = el.timer_index;
      addrDone = el.done_address ? formatAddress(el.done_address) : "Q1";
      if (el.type === "rto") {
        addrReset = el.reset_address ? formatAddress(el.reset_address) : "I4";
      }
    }
    if (el.type === "ctu") {
      presetC = el.preset;
      counterIndex = el.counter_index;
      addrDone = el.done_address ? formatAddress(el.done_address) : "Q2";
      addrReset = el.reset_address ? formatAddress(el.reset_address) : "I3";
    }
    if (el.type === "ctd") {
      presetC = el.preset;
      counterIndex = el.counter_index;
      addrDone = el.done_address ? formatAddress(el.done_address) : "Q2";
      addrReset = el.load_address ? formatAddress(el.load_address) : "I5";
    }
    if (el.type === "math") {
      mathOp = el.op;
      addrA = formatAddress(el.a);
      addrB = formatAddress(el.b);
      addrDest = formatAddress(el.dest);
    }
    if (el.type === "move") {
      addrA = formatAddress(el.source);
      addrDest = formatAddress(el.dest);
    }
    if (el.type === "compare") {
      cmpOp = el.op;
      addrA = formatAddress(el.a);
      addrB = formatAddress(el.b);
    }
  }

  function applyQuick() {
    const p = parseVarString(quick);
    if (!p) {
      parseError = "Invalid address. Examples: I0, Q1, M2.3, R10, R1.5";
      return;
    }
    parseError = "";
    prefix = p.prefix === "M" ? "M" : p.prefix;
    // normalize M/R for holding
    if (p.address.area === "holding") {
      prefix = p.bit != null || p.prefix === "M" ? (p.prefix === "M" ? "M" : "R") : "R";
    }
    index = p.index;
    bit = p.bit ?? 0;
    useBit = p.bit != null || p.prefix === "M";
    quick = p.display;
  }

  function primaryAddress(): Address {
    return formToAddress(prefix, index, useBit ? bit : null, useBit || prefix === "M");
  }

  function mustParse(s: string, label: string): Address | null {
    const p = parseVarString(s);
    if (!p) {
      parseError = `Invalid address (${label}): ${s}`;
      return null;
    }
    return p.address;
  }

  function apply() {
    parseError = "";
    let next: LadderElement = { ...element } as LadderElement;

    if (isBit && "address" in next) {
      next = {
        type: selectedType,
        id: element.id,
        address: primaryAddress(),
      } as LadderElement;
    } else if (isTimer && (next.type === "ton" || next.type === "tof" || next.type === "rto")) {
      const done = mustParse(addrDone, "done");
      if (!done) return;
      next = {
        ...next,
        preset_ms: Math.max(1, presetMs),
        timer_index: Math.max(0, timerIndex),
        done_address: done,
      };
      if (next.type === "rto") {
        const rst = mustParse(addrReset, "reset");
        if (!rst) return;
        next = { ...next, reset_address: rst };
      }
    } else if (next.type === "ctu") {
      const done = mustParse(addrDone, "done");
      const rst = mustParse(addrReset, "reset");
      if (!done || !rst) return;
      next = {
        ...next,
        preset: Math.max(1, presetC),
        counter_index: Math.max(0, counterIndex),
        done_address: done,
        reset_address: rst,
      };
    } else if (next.type === "ctd") {
      const done = mustParse(addrDone, "done");
      const ld = mustParse(addrReset, "load");
      if (!done || !ld) return;
      next = {
        ...next,
        preset: Math.max(1, presetC),
        counter_index: Math.max(0, counterIndex),
        done_address: done,
        load_address: ld,
      };
    } else if (next.type === "math") {
      const a = mustParse(addrA, "A");
      const b = mustParse(addrB, "B");
      const d = mustParse(addrDest, "dest");
      if (!a || !b || !d) return;
      next = { ...next, op: mathOp, a, b, dest: d };
    } else if (next.type === "move") {
      const s = mustParse(addrA, "source");
      const d = mustParse(addrDest, "dest");
      if (!s || !d) return;
      next = { ...next, source: s, dest: d };
    } else if (next.type === "compare") {
      const a = mustParse(addrA, "A");
      const b = mustParse(addrB, "B");
      if (!a || !b) return;
      next = { ...next, op: cmpOp, a, b };
    } else if (isWire) {
      // nothing
    }

    onApply(next, labelInput.trim().slice(0, 10));
    onClose();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (showHelp) showHelp = false;
      else onClose();
    }
  }

  const typeLabel = $derived(element.type.replaceAll("_", " ").toUpperCase());
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={onClose} onkeydown={onKey} role="presentation">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="dlg"
      onclick={(e) => e.stopPropagation()}
      onkeydown={onKey}
      role="dialog"
      aria-modal="true"
      aria-labelledby="dlg-title"
      tabindex="-1"
    >
      <header class="dlg-h">
        <div>
          <h2 id="dlg-title">Element properties</h2>
          <p class="sub">{typeLabel} · <code>{element.id}</code></p>
        </div>
        <div class="hdr-actions">
          <button type="button" class="tia-btn" onclick={() => (showHelp = !showHelp)}>
            {showHelp ? "← Back" : "Help"}
          </button>
          <button type="button" class="tia-btn" onclick={onClose}>✕</button>
        </div>
      </header>

      {#if showHelp}
        <div class="help-body">
          {#each ADDRESS_HELP_MD.split("\n") as line}
            {#if line.startsWith("# ")}
              <h3>{line.slice(2)}</h3>
            {:else if line.startsWith("## ")}
              <h4>{line.slice(3)}</h4>
            {:else if line.startsWith("|")}
              <div class="md-row">{line}</div>
            {:else if line.startsWith("```")}
              <!-- skip fence -->
            {:else if line.trim() === ""}
              <div class="sp"></div>
            {:else}
              <p>{line}</p>
            {/if}
          {/each}
        </div>
      {:else}
        <div class="dlg-b">
          <section class="sec">
            <h3>Symbol / label (max 10 chars)</h3>
            <div class="row">
              <input
                class="grow"
                maxlength="10"
                placeholder="e.g. BTN_START, M1_on, Motor_Run"
                bind:value={labelInput}
              />
              <span class="muted">shown above the element</span>
            </div>
          </section>

          {#if isBit}
            <section class="sec">
              <h3>Element type</h3>
              <div class="row">
                <label class="grow">
                  Type
                  <select bind:value={selectedType}>
                    {#if isContactFamily}
                      <optgroup label="Contacts">
                        <option value="contact_no">—│ │—  NO contact</option>
                        <option value="contact_nc">—│/│—  NC contact</option>
                        <option value="contact_rising">—│P│—  Rising edge ↑</option>
                        <option value="contact_falling">—│N│—  Falling edge ↓</option>
                      </optgroup>
                    {/if}
                    {#if isCoilFamily}
                      <optgroup label="Coils / outputs">
                        <option value="coil">—( )—  Coil</option>
                        <option value="coil_negated">—(/)—  Negated coil</option>
                        <option value="coil_set">—(S)—  SET (latch)</option>
                        <option value="coil_reset">—(R)—  RESET (unlatch)</option>
                      </optgroup>
                    {/if}
                  </select>
                </label>
              </div>
            </section>

            <section class="sec">
              <h3>Variable address (I / Q / M / R)</h3>
              <div class="row">
                <label>
                  Prefix
                  <select bind:value={prefix}>
                    <option value="I">I — Input</option>
                    <option value="Q">Q — Output</option>
                    <option value="M">M — Memory bit</option>
                    <option value="R">R — Register</option>
                  </select>
                </label>
                <label>
                  Number
                  <input type="number" min="0" max="65535" bind:value={index} />
                </label>
                <label class="chk">
                  <input
                    type="checkbox"
                    bind:checked={useBit}
                    disabled={prefix === "I" || prefix === "Q"}
                  />
                  Bit (0–15)
                </label>
                <label>
                  Bit
                  <input
                    type="number"
                    min="0"
                    max="15"
                    bind:value={bit}
                    disabled={!useBit && prefix !== "M"}
                  />
                </label>
              </div>
              <p class="preview">
                Preview: <strong>{formatAddress(primaryAddress())}</strong>
              </p>
            </section>

            <section class="sec">
              <h3>Quick address</h3>
              <div class="row">
                <input
                  class="grow"
                  placeholder="e.g. I0, Q1, M2.3, R10, R1.5"
                  bind:value={quick}
                  onkeydown={(e) => e.key === "Enter" && applyQuick()}
                />
                <button type="button" class="tia-btn" onclick={applyQuick}>Parse</button>
              </div>
            </section>
          {:else if isTimer}
            <section class="sec">
              <h3>Timer</h3>
              <div class="row">
                <label>PT [ms] <input type="number" min="1" bind:value={presetMs} /></label>
                <label>Timer index <input type="number" min="0" bind:value={timerIndex} /></label>
                <label class="grow">Done (Q/M/R.x) <input bind:value={addrDone} /></label>
                {#if element.type === "rto"}
                  <label class="grow">Reset <input bind:value={addrReset} /></label>
                {/if}
              </div>
            </section>
          {:else if isCounter}
            <section class="sec">
              <h3>Counter</h3>
              <div class="row">
                <label>PV <input type="number" min="1" bind:value={presetC} /></label>
                <label>Counter index <input type="number" min="0" bind:value={counterIndex} /></label>
                <label class="grow">Done <input bind:value={addrDone} /></label>
                <label class="grow"
                  >{element.type === "ctd" ? "Load" : "Reset"}
                  <input bind:value={addrReset} /></label
                >
              </div>
            </section>
          {:else if isMath}
            <section class="sec">
              <h3>Math</h3>
              <div class="row">
                <label>
                  Op
                  <select bind:value={mathOp}>
                    <option value="add">ADD</option>
                    <option value="sub">SUB</option>
                    <option value="mul">MUL</option>
                    <option value="div">DIV</option>
                  </select>
                </label>
                <label class="grow">A <input bind:value={addrA} placeholder="R0" /></label>
                <label class="grow">B <input bind:value={addrB} placeholder="R1" /></label>
                <label class="grow">Dest <input bind:value={addrDest} placeholder="R2" /></label>
              </div>
            </section>
          {:else if isMove}
            <section class="sec">
              <h3>MOVE</h3>
              <div class="row">
                <label class="grow">Source <input bind:value={addrA} /></label>
                <label class="grow">Dest <input bind:value={addrDest} /></label>
              </div>
            </section>
          {:else if isCmp}
            <section class="sec">
              <h3>Compare</h3>
              <div class="row">
                <label>
                  Op
                  <select bind:value={cmpOp}>
                    <option value="eq">==</option>
                    <option value="ne">&lt;&gt;</option>
                    <option value="gt">&gt;</option>
                    <option value="ge">&gt;=</option>
                    <option value="lt">&lt;</option>
                    <option value="le">&lt;=</option>
                  </select>
                </label>
                <label class="grow">A <input bind:value={addrA} /></label>
                <label class="grow">B <input bind:value={addrB} /></label>
              </div>
            </section>
          {:else}
            <p class="muted">Ten element nie ma konfigurowalnego adresu.</p>
          {/if}

          {#if parseError}
            <p class="err">{parseError}</p>
          {/if}
        </div>

        <footer class="dlg-f">
          <button type="button" class="tia-btn" onclick={() => (showHelp = true)}>Help</button>
          <div class="spacer"></div>
          <button type="button" class="tia-btn" onclick={onClose}>Anuluj</button>
          <button type="button" class="tia-btn tia-btn-primary" onclick={apply}>Zastosuj</button>
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 99999;
    background: rgba(15, 25, 35, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
  }
  .dlg {
    width: min(560px, 100%);
    max-height: min(86vh, 720px);
    display: flex;
    flex-direction: column;
    background: #f4f6f8;
    border: 1px solid #7a8a99;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
    border-radius: 3px;
  }
  .dlg-h {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
    background: linear-gradient(180deg, #2a4a62, #1a3a52);
    color: #fff;
  }
  .dlg-h h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
  }
  .sub {
    margin: 4px 0 0;
    font-size: 11px;
    opacity: 0.85;
  }
  .sub code {
    font-family: Consolas, monospace;
  }
  .hdr-actions {
    display: flex;
    gap: 6px;
  }
  .dlg-b {
    padding: 12px 14px;
    overflow: auto;
    flex: 1;
  }
  .sec {
    margin-bottom: 14px;
    background: #fff;
    border: 1px solid #c5ccd2;
    padding: 10px 12px;
    border-radius: 2px;
  }
  .sec h3 {
    margin: 0 0 8px;
    font-size: 12px;
    color: #005f87;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: flex-end;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 11px;
    color: #5a6570;
    min-width: 72px;
  }
  label.grow {
    flex: 1;
    min-width: 120px;
  }
  label.chk {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    padding-bottom: 6px;
  }
  input,
  select {
    font-size: 12px;
    padding: 5px 7px;
    border: 1px solid #9aa3ab;
    border-radius: 2px;
    font-family: Consolas, monospace;
  }
  .grow {
    flex: 1;
  }
  .preview {
    margin: 10px 0 0;
    font-size: 12px;
  }
  .preview strong {
    font-family: Consolas, monospace;
    color: #003d5c;
    font-size: 14px;
  }
  .err {
    color: #c00000;
    font-size: 12px;
    margin: 0 0 8px;
  }
  .muted {
    color: #5a6570;
    font-size: 12px;
  }
  .dlg-f {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-top: 1px solid #c5ccd2;
    background: #e8eef2;
  }
  .spacer {
    flex: 1;
  }
  .help-body {
    padding: 12px 16px 20px;
    overflow: auto;
    flex: 1;
    background: #fff;
    font-size: 12px;
    line-height: 1.45;
    color: #222;
  }
  .help-body h3 {
    margin: 0 0 10px;
    color: #005f87;
  }
  .help-body h4 {
    margin: 14px 0 6px;
    color: #003d5c;
    font-size: 12px;
  }
  .help-body p {
    margin: 0 0 6px;
  }
  .md-row {
    font-family: Consolas, monospace;
    font-size: 11px;
    white-space: pre-wrap;
    color: #333;
  }
  .sp {
    height: 6px;
  }
</style>
