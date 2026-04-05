[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / createWorkerSession

# Function: createWorkerSession()

> **createWorkerSession**(`canvas`, `options?`): `Promise`\<[`WorkerSession`](../classes/WorkerSession.md)\>

Defined in: [index.ts:1927](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1927)

Create a worker-backed canvas session with optional main-thread fallback.

This is the preferred path for heavier interactive views when the browser
supports `Worker` plus `OffscreenCanvas`.

## Parameters

### canvas

`HTMLCanvasElement`

### options?

[`WorkerSessionOptions`](../../shared/interfaces/WorkerSessionOptions.md)

## Returns

`Promise`\<[`WorkerSession`](../classes/WorkerSession.md)\>
