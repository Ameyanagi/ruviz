[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / WorkerSession

# Class: WorkerSession

Defined in: [index.ts:1475](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1475)

Worker-backed interactive canvas session with main-thread fallback support.

## Constructors

### Constructor

> **new WorkerSession**(`canvas`, `mode`, `fallbackSession?`): `WorkerSession`

Defined in: [index.ts:1490](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1490)

#### Parameters

##### canvas

`HTMLCanvasElement`

##### mode

[`SessionMode`](../../shared/type-aliases/SessionMode.md)

##### fallbackSession?

[`CanvasSession`](CanvasSession.md)

#### Returns

`WorkerSession`

## Properties

### mode

> `readonly` **mode**: [`SessionMode`](../../shared/type-aliases/SessionMode.md)

Defined in: [index.ts:1476](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1476)

## Accessors

### canvas

#### Get Signature

> **get** **canvas**(): `HTMLCanvasElement`

Defined in: [index.ts:1512](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1512)

##### Returns

`HTMLCanvasElement`

## Methods

### \_pushCleanup()

> **\_pushCleanup**(`dispose`): `void`

Defined in: [index.ts:1747](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1747)

#### Parameters

##### dispose

() => `void`

#### Returns

`void`

***

### attachWorker()

> **attachWorker**(`worker`): `void`

Defined in: [index.ts:1516](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1516)

#### Parameters

##### worker

`Worker`

#### Returns

`void`

***

### destroy()

> **destroy**(): `void`

Defined in: [index.ts:1716](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1716)

#### Returns

`void`

***

### dispose()

> **dispose**(): `void`

Defined in: [index.ts:1728](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1728)

#### Returns

`void`

***

### exportPng()

> **exportPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [index.ts:1686](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1686)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### exportSvg()

> **exportSvg**(): `Promise`\<`string`\>

Defined in: [index.ts:1699](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1699)

#### Returns

`Promise`\<`string`\>

***

### hasPlot()

> **hasPlot**(): `boolean`

Defined in: [index.ts:1527](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1527)

#### Returns

`boolean`

***

### pointerDown()

> **pointerDown**(`x`, `y`, `button`): `void`

Defined in: [index.ts:1621](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1621)

#### Parameters

##### x

`number`

##### y

`number`

##### button

`number`

#### Returns

`void`

***

### pointerLeave()

> **pointerLeave**(): `void`

Defined in: [index.ts:1660](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1660)

#### Returns

`void`

***

### pointerMove()

> **pointerMove**(`x`, `y`): `void`

Defined in: [index.ts:1634](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1634)

#### Parameters

##### x

`number`

##### y

`number`

#### Returns

`void`

***

### pointerUp()

> **pointerUp**(`x`, `y`, `button`): `void`

Defined in: [index.ts:1647](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1647)

#### Parameters

##### x

`number`

##### y

`number`

##### button

`number`

#### Returns

`void`

***

### ready()

> **ready**(): `Promise`\<`void`\>

Defined in: [index.ts:1712](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1712)

#### Returns

`Promise`\<`void`\>

***

### render()

> **render**(): `void`

Defined in: [index.ts:1595](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1595)

#### Returns

`void`

***

### resetView()

> **resetView**(): `void`

Defined in: [index.ts:1608](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1608)

#### Returns

`void`

***

### resize()

> **resize**(`width?`, `height?`, `scaleFactor?`): `void`

Defined in: [index.ts:1557](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1557)

#### Parameters

##### width?

`number`

##### height?

`number`

##### scaleFactor?

`number`

#### Returns

`void`

***

### setBackendPreference()

> **setBackendPreference**(`backendPreference`): `void`

Defined in: [index.ts:1584](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1584)

#### Parameters

##### backendPreference

[`BackendPreference`](../../shared/type-aliases/BackendPreference.md)

#### Returns

`void`

***

### setPlot()

> **setPlot**(`plot`): `Promise`\<`void`\>

Defined in: [index.ts:1535](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1535)

#### Parameters

##### plot

[`PlotBuilder`](PlotBuilder.md)

#### Returns

`Promise`\<`void`\>

***

### setTime()

> **setTime**(`timeSeconds`): `void`

Defined in: [index.ts:1571](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1571)

#### Parameters

##### timeSeconds

`number`

#### Returns

`void`

***

### wheel()

> **wheel**(`deltaY`, `x`, `y`): `void`

Defined in: [index.ts:1673](https://github.com/Ameyanagi/ruviz/blob/2ea97bee578b78d3002281618aecdb4fafa6ecec/packages/ruviz-web/src/index.ts#L1673)

#### Parameters

##### deltaY

`number`

##### x

`number`

##### y

`number`

#### Returns

`void`
