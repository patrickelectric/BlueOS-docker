// The example service as a surface computer sees it, over the vehicle's Zenoh WebSocket. The service, zenohd and its
// remote_api plugin run on the vehicle; this file only talks to them. The messages are read from the IDL files.
//
//   cd core/services/example && BLUEOS_ADDRESS=blueos.local bun tests/example.ts
//
// It is a plain script and not a `bun test` file because only `bun run` installs the pinned imports on its own, as
// long as no node_modules folder is above the working directory.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import type { ChannelReceiver, Reply, Session, Subscriber } from "@eclipse-zenoh/zenoh-ts@1.9.0";
import { parse } from "@foxglove/rosmsg@5.0.5";
import { MessageReader, MessageWriter } from "@foxglove/rosmsg2-serialization@3.0.3";

// zenoh-ts imports its wasm-bindgen modules as ES modules, which Bun does not instantiate.
Bun.plugin({
  name: "wasm-bindgen",
  setup(build) {
    build.onLoad({ filter: /\.wasm$/ }, async ({ path }) => {
      const module = new WebAssembly.Module(await Bun.file(path).arrayBuffer());
      const imports = [...new Set(WebAssembly.Module.imports(module).map((entry) => entry.module))];
      const contents = [
        ...imports.map((name, index) => `import * as import${index} from ${JSON.stringify(name)};`),
        `const { instance } = await WebAssembly.instantiate(await Bun.file(${JSON.stringify(path)}).arrayBuffer(), {`,
        ...imports.map((name, index) => `  ${JSON.stringify(name)}: import${index},`),
        "});",
        ...WebAssembly.Module.exports(module).map(({ name }) => `export const ${name} = instance.exports.${name};`),
      ];
      return { loader: "js", contents: contents.join("\n") };
    });
  },
});
const { Config, ReplyError, Session: Sessions } = await import("@eclipse-zenoh/zenoh-ts@1.9.0");

const ADDRESS = process.env.BLUEOS_ADDRESS ?? "blueos.local";
const IDL = `${import.meta.dir}/../../../libs/idl`;
const RESULTS = "blueos/v1/example/jobs/SetLevel/result";
const TIMEOUT_MS = 5000;

const SECTIONS: Record<string, number> = { Goal: 0, Result: 1, Feedback: 2, Request: 0, Response: 1 };

// "pkg/action/Name_Goal" is the first section of pkg/action/Name.action, "pkg/msg/Name" is pkg/msg/Name.msg.
function source(schema: string): string {
  const [, pkg, kind, name, part] = schema.match(/^(\w+)\/(msg|srv|action)\/([A-Za-z0-9]+)(?:_(\w+))?$/) ?? [];
  if (!pkg) {
    throw new Error(`not a schema name: ${schema}`);
  }
  const text = readFileSync(`${IDL}/interfaces/${pkg}/${kind}/${name}.${kind}`, "utf8");
  return part === undefined ? text : text.split(/^---$/m)[SECTIONS[part]];
}

// The message and every nested type it uses, as one ROS 2 definition text.
function definition(schema: string) {
  const sections = [source(schema)];
  const seen = new Set<string>();
  for (let index = 0; index < sections.length; index++) {
    for (const [, nested] of sections[index].matchAll(/^\s*(\w+\/\w+)(?:\[\d*\])?\s+\w+/gm)) {
      if (!seen.has(nested)) {
        seen.add(nested);
        const [pkg, name] = nested.split("/");
        sections.push(`${"=".repeat(80)}\nMSG: ${nested}\n${source(`${pkg}/msg/${name}`)}`);
      }
    }
  }
  return parse(sections.join("\n"), { ros2: true });
}

const encode = (schema: string, value: object) => new MessageWriter(definition(schema)).writeMessage(value);
const decode = <T>(schema: string, bytes: Uint8Array) => new MessageReader(definition(schema)).readMessage<T>(bytes);
const constant = (schema: string, name: string) =>
  definition(schema)[0].definitions.find((field) => field.isConstant && field.name === name)?.value;

const STATUS_EXECUTING = constant("blueos_msgs/msg/JobStatus", "STATUS_EXECUTING");
const STATUS_SUCCEEDED = constant("blueos_msgs/msg/JobStatus", "STATUS_SUCCEEDED");
const STATUS_CANCELED = constant("blueos_msgs/msg/JobStatus", "STATUS_CANCELED");

type CommandAck = { accepted: boolean; job_id: string; status: number; reason: string };
type JobResult = { job: { job_id: string; job_type: string; status: number; reason: string }; result: Uint8Array };
type PumpState = { level: number; max_level: number };

async function within<T>(promise: Promise<T>, what: string): Promise<T> {
  let timer: Timer | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${what}: no answer in ${TIMEOUT_MS} ms`)), TIMEOUT_MS);
  });
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer));
}

let session: Session;
let results: Subscriber;

async function reply(receiver: ChannelReceiver<Reply> | undefined, what: string): Promise<Uint8Array> {
  if (!receiver) {
    throw new Error(`${what}: the query was not sent`);
  }
  const answer = await within(receiver.receive(), what).catch(() => {
    throw new Error(`${what}: no reply, is the example running?`);
  });
  const result = answer.result();
  if (result instanceof ReplyError) {
    throw new Error(`${what}: ${result.payload().toString()}`);
  }
  return result.payload().toBytes();
}

async function command(name: string, jobId: string, level?: number): Promise<CommandAck> {
  const payload = level === undefined ? {} : { payload: encode("blueos_example_msgs/action/SetLevel_Goal", { level }) };
  const receiver = await session.get(`blueos/v1/example/command/${name}`, { attachment: jobId, ...payload });
  return decode("blueos_msgs/msg/CommandAck", await reply(receiver, `command/${name}`));
}

async function query<T>(key: string, schema: string): Promise<T> {
  return decode(schema, await reply(await session.get(`blueos/v1/example/${key}`), key));
}

const level = () => query<PumpState>("query/Level", "blueos_example_msgs/srv/Level_Response");

async function resultOf(jobId: string): Promise<JobResult> {
  const receiver = results.receiver();
  assert.ok(receiver, "the results subscriber has no receiver");
  for (;;) {
    const sample = await within(receiver.receive(), `the result of ${jobId}`);
    const result = decode<JobResult>("blueos_msgs/msg/JobResult", sample.payload().toBytes());
    if (result.job.job_id === jobId) {
      return result;
    }
  }
}

const offline: [string, () => void][] = [
  ["CDR matches the Rust side", () => {
    const used = /SetLevel_(Goal|Result|Feedback)$|PumpState$|Level_Response$|CommandAck$|JobResult$/;
    const { vectors } = JSON.parse(readFileSync(`${IDL}/tests/vectors/cdr.json`, "utf8"));
    const checked = vectors.filter(({ schema_name }: { schema_name: string }) => used.test(schema_name));
    assert.ok(checked.length >= 7, `only ${checked.length} vectors`);
    for (const { schema_name, hex, decoded } of checked) {
      const value = decode<Record<string, unknown>>(schema_name, Uint8Array.from(Buffer.from(hex, "hex")));
      const plain = JSON.parse(JSON.stringify(value, (_, field) => (field instanceof Uint8Array ? [...field] : field)));
      assert.deepEqual(plain, decoded, schema_name);
      assert.equal(Buffer.from(encode(schema_name, value)).toString("hex"), hex, schema_name);
    }
  }],
];

const online: [string, () => Promise<void>][] = [
  ["the example is alive and answers the Level query", async () => {
    await reply(await session.liveliness().get("blueos/v1/services/example"), "liveliness");
    assert.ok((await level()).max_level > 0);
  }],
  ["a level above the maximum is refused", async () => {
    const { max_level } = await level();
    if (max_level === 255) {
      return;
    }
    const ack = await command("SetLevel", crypto.randomUUID(), max_level + 1);
    assert.deepEqual([ack.accepted, ack.reason], [false, "the level is above the maximum"]);
  }],
  ["a SetLevel Job moves the pump and ends with its result", async () => {
    const { level: start, max_level } = await level();
    const target = start < max_level ? start + 1 : start - 1;
    const job = crypto.randomUUID();

    const ack = await command("SetLevel", job, target);
    assert.deepEqual([ack.accepted, ack.status, ack.reason], [true, STATUS_EXECUTING, ""]);

    const { job: status, result } = await resultOf(job);
    assert.equal(status.status, STATUS_SUCCEEDED);
    assert.equal(decode<{ level: number }>("blueos_example_msgs/action/SetLevel_Result", result).level, target);
    assert.equal((await query<PumpState>("state/pump", "blueos_example_msgs/msg/PumpState")).level, target);
    assert.equal((await level()).level, target);
  }],
  ["a cancel stops the pump", async () => {
    const { level: start, max_level } = await level();
    const job = crypto.randomUUID();

    assert.ok((await command("SetLevel", job, start <= max_level / 2 ? max_level : 0)).accepted);
    const cancelled = await command("CancelJob", job);
    assert.deepEqual([cancelled.accepted, cancelled.status], [true, STATUS_CANCELED]);
    assert.equal((await resultOf(job)).job.status, STATUS_CANCELED);
  }],
];

let failed = 0;
async function run(name: string, check: () => void | Promise<void>) {
  try {
    await check();
    console.log(`ok   ${name}`);
  } catch (error) {
    failed++;
    console.log(`FAIL ${name}\n     ${error instanceof Error ? error.message : error}`);
  }
}

for (const [name, check] of offline) {
  await run(name, check);
}
await run(`connect to Zenoh on ${ADDRESS}`, async () => {
  session = await within(Sessions.open(new Config(`ws://${ADDRESS}/zenoh-api/`)), `Zenoh on ${ADDRESS}`);
  results = await session.declareSubscriber(RESULTS);
});
if (session) {
  for (const [name, check] of online) {
    await run(name, check);
  }
  await results?.undeclare();
  await session.close();
}
process.exit(failed === 0 ? 0 : 1);
