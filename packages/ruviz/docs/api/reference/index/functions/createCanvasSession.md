[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / createCanvasSession

# Function: createCanvasSession()

> **createCanvasSession**(`canvas`, `options?`): `Promise`\<[`CanvasSession`](../classes/CanvasSession.md)\>

Defined in: [index.ts:2781](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2781)

Create an interactive main-thread canvas session.

Use this when you want explicit control over session setup rather than
calling `plot.mount(canvas)`.

## Parameters

### canvas

`HTMLCanvasElement`

### options?

[`CanvasSessionOptions`](../../shared/interfaces/CanvasSessionOptions.md)

## Returns

`Promise`\<[`CanvasSession`](../classes/CanvasSession.md)\>
