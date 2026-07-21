<script lang="ts">
  import { plc } from "../../shared/stores/plc.svelte";
</script>

<div class="lad-paper">
  <div class="tia-page-header">
    <div>
      <h1>Mathematical operations</h1>
      <p>IEC-style blocks available in the ladder palette (drag into a network).</p>
    </div>
    <button type="button" class="tia-btn tia-btn-primary" onclick={() => plc.setView("ladder")}
      >Open ladder editor</button
    >
  </div>

  <div class="tia-table-wrap">
    <table class="tia-table">
      <thead>
        <tr>
          <th>Block</th>
          <th>Operation</th>
          <th>Operands</th>
          <th>Notes</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td><strong>ADD</strong></td>
          <td>dest = a + b</td>
          <td>R / MR / IW words</td>
          <td>Overflow → PLC FAULT/STOP</td>
        </tr>
        <tr>
          <td><strong>SUB</strong></td>
          <td>dest = a − b</td>
          <td>R / MR / IW</td>
          <td>Checked arithmetic</td>
        </tr>
        <tr>
          <td><strong>MUL</strong></td>
          <td>dest = a × b</td>
          <td>R / MR / IW</td>
          <td>Overflow → FAULT</td>
        </tr>
        <tr>
          <td><strong>DIV</strong></td>
          <td>dest = a ÷ b</td>
          <td>R / MR / IW</td>
          <td>Divide-by-zero → FAULT</td>
        </tr>
        <tr>
          <td><strong>MOVE</strong></td>
          <td>dest := source</td>
          <td>Words or bits</td>
          <td>Executes when power flow TRUE</td>
        </tr>
        <tr>
          <td><strong>CMP</strong></td>
          <td>== &lt;&gt; &gt; &gt;= &lt; &lt;=</td>
          <td>Two addresses</td>
          <td>Result AND-ed into power rail</td>
        </tr>
      </tbody>
    </table>

    <p style="margin-top:16px;font-size:12px;color:var(--tia-muted);line-height:1.5">
      Demo network 4: set <code>R40</code> ≥ <code>R41</code> (Watch / process image), then RUN — MOVE copies
      to <code>R42</code> and sets <code>Q3</code>.
    </p>
  </div>
</div>
