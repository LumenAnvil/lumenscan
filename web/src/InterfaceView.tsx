// Renders a decoded Interface. The layout mirrors the CLI's sections, but as
// real HTML: a card per declared item, signatures as code, doc comments as
// prose.

import type {
  Enum,
  ErrorEnum,
  Event,
  Func,
  Interface,
  Struct,
  Union,
} from "./model";
import { isEmptyInterface } from "./model";

function Section(props: {
  title: string;
  count: number;
  children: React.ReactNode;
}) {
  if (props.count === 0) return null;
  return (
    <section className="section">
      <h2>
        {props.title} <span className="count">{props.count}</span>
      </h2>
      {props.children}
    </section>
  );
}

function Doc({ text }: { text: string }) {
  const trimmed = text.trim();
  if (!trimmed) return null;
  return <p className="doc">{trimmed}</p>;
}

function signature(f: Func): string {
  const args = f.inputs.map((i) => `${i.name}: ${i.type}`).join(", ");
  const ret = f.outputs.length > 0 ? ` -> ${f.outputs[0]}` : "";
  return `${f.name}(${args})${ret}`;
}

function FunctionCard({ f }: { f: Func }) {
  return (
    <div className="card">
      <code className="sig">{signature(f)}</code>
      <Doc text={f.doc} />
    </div>
  );
}

function StructCard({ s }: { s: Struct }) {
  return (
    <div className="card">
      <code className="name">{s.name}</code>
      <Doc text={s.doc} />
      <table>
        <thead>
          <tr>
            <th>Field</th>
            <th>Type</th>
          </tr>
        </thead>
        <tbody>
          {s.fields.map((field) => (
            <tr key={field.name}>
              <td>
                <code>{field.name}</code>
              </td>
              <td>
                <code>{field.type}</code>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function UnionCard({ u }: { u: Union }) {
  return (
    <div className="card">
      <code className="name">{u.name}</code>
      <Doc text={u.doc} />
      <ul className="cases">
        {u.cases.map((c) => (
          <li key={c.name}>
            <code>
              {c.types.length === 0
                ? c.name
                : `${c.name}(${c.types.join(", ")})`}
            </code>
            {c.doc.trim() && <span className="case-doc"> — {c.doc.trim()}</span>}
          </li>
        ))}
      </ul>
    </div>
  );
}

function EnumCard({ e, label }: { e: Enum | ErrorEnum; label: string }) {
  return (
    <div className="card">
      <code className="name">{e.name}</code>
      <Doc text={e.doc} />
      <table>
        <thead>
          <tr>
            <th>{label}</th>
            <th>Value</th>
          </tr>
        </thead>
        <tbody>
          {e.cases.map((c) => (
            <tr key={c.name}>
              <td>
                <code>{c.name}</code>
              </td>
              <td>
                <code>{c.value}</code>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function EventCard({ ev }: { ev: Event }) {
  return (
    <div className="card">
      <code className="name">{ev.name}</code>
      <Doc text={ev.doc} />
      <div className="event-meta">
        {ev.prefixTopics.length > 0 && (
          <span>
            topics:{" "}
            {ev.prefixTopics.map((t) => (
              <code key={t}>{t}</code>
            ))}
          </span>
        )}
        <span>
          data format: <code>{ev.dataFormat}</code>
        </span>
      </div>
      {ev.params.length > 0 && (
        <table>
          <thead>
            <tr>
              <th>Parameter</th>
              <th>Type</th>
              <th>Location</th>
            </tr>
          </thead>
          <tbody>
            {ev.params.map((p) => (
              <tr key={p.name}>
                <td>
                  <code>{p.name}</code>
                </td>
                <td>
                  <code>{p.type}</code>
                </td>
                <td>{p.location}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

export function InterfaceView({ iface }: { iface: Interface }) {
  if (isEmptyInterface(iface)) {
    return (
      <p className="empty">This contract declares no functions or types.</p>
    );
  }

  return (
    <div className="interface">
      <Section title="Functions" count={iface.functions.length}>
        {iface.functions.map((f) => (
          <FunctionCard key={f.name} f={f} />
        ))}
      </Section>

      <Section title="Structs" count={iface.structs.length}>
        {iface.structs.map((s) => (
          <StructCard key={s.name} s={s} />
        ))}
      </Section>

      <Section title="Unions" count={iface.unions.length}>
        {iface.unions.map((u) => (
          <UnionCard key={u.name} u={u} />
        ))}
      </Section>

      <Section title="Enums" count={iface.enums.length}>
        {iface.enums.map((e) => (
          <EnumCard key={e.name} e={e} label="Variant" />
        ))}
      </Section>

      <Section title="Error enums" count={iface.errorEnums.length}>
        {iface.errorEnums.map((e) => (
          <EnumCard key={e.name} e={e} label="Variant" />
        ))}
      </Section>

      <Section title="Events" count={iface.events.length}>
        {iface.events.map((ev) => (
          <EventCard key={ev.name} ev={ev} />
        ))}
      </Section>
    </div>
  );
}
