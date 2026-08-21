[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / WorkerSession

# Class: WorkerSession

Defined in: [index.ts:2262](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2262)

Worker-backed interactive canvas session with main-thread fallback support.

## Constructors

### Constructor

> **new WorkerSession**(`canvas`, `mode`, `fallbackSession?`): `WorkerSession`

Defined in: [index.ts:2279](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2279)

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

Defined in: [index.ts:2263](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2263)

## Accessors

### canvas

#### Get Signature

> **get** **canvas**(): `HTMLCanvasElement`

Defined in: [index.ts:2303](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2303)

##### Returns

`HTMLCanvasElement`

## Methods

### \_pushCleanup()

> **\_pushCleanup**(`dispose`): `void`

Defined in: [index.ts:2644](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2644)

#### Parameters

##### dispose

() => `void`

#### Returns

`void`

***

### attachWorker()

> **attachWorker**(`worker`): `void`

Defined in: [index.ts:2307](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2307)

#### Parameters

##### worker

`Worker`

#### Returns

`void`

***

### destroy()

> **destroy**(): `void`

Defined in: [index.ts:2605](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2605)

#### Returns

`void`

***

### dispose()

> **dispose**(): `void`

Defined in: [index.ts:2621](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2621)

#### Returns

`void`

***

### exportPng()

> **exportPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [index.ts:2575](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2575)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### exportSvg()

> **exportSvg**(): `Promise`\<`string`\>

Defined in: [index.ts:2588](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2588)

#### Returns

`Promise`\<`string`\>

***

### hasPlot()

> **hasPlot**(): `boolean`

Defined in: [index.ts:2318](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2318)

#### Returns

`boolean`

***

### hitAt()

> **hitAt**(`x`, `y`): `Promise`\<[`SeriesHit`](../../shared/interfaces/SeriesHit.md) \| `null`\>

Defined in: [index.ts:2491](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2491)

The series data point under a canvas pixel, or `null`. Asynchronous
because a worker session answers from the worker; the main-thread
fallback resolves immediately.

#### Parameters

##### x

`number`

##### y

`number`

#### Returns

`Promise`\<[`SeriesHit`](../../shared/interfaces/SeriesHit.md) \| `null`\>

***

### legendEntryAt()

> **legendEntryAt**(`x`, `y`): `Promise`\<`number` \| `null`\>

Defined in: [index.ts:2505](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2505)

The index of the series behind a legend entry at a canvas pixel, or `null`.

#### Parameters

##### x

`number`

##### y

`number`

#### Returns

`Promise`\<`number` \| `null`\>

***

### pointerDown()

> **pointerDown**(`x`, `y`, `button`): `void`

Defined in: [index.ts:2421](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2421)

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

Defined in: [index.ts:2460](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2460)

#### Returns

`void`

***

### pointerMove()

> **pointerMove**(`x`, `y`): `void`

Defined in: [index.ts:2434](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2434)

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

Defined in: [index.ts:2447](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2447)

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

Defined in: [index.ts:2601](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2601)

#### Returns

`Promise`\<`void`\>

***

### render()

> **render**(): `void`

Defined in: [index.ts:2395](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2395)

#### Returns

`void`

***

### resetView()

> **resetView**(): `void`

Defined in: [index.ts:2408](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2408)

#### Returns

`void`

***

### resize()

> **resize**(`width?`, `height?`, `scaleFactor?`): `void`

Defined in: [index.ts:2357](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2357)

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

### seriesCount()

> **seriesCount**(): `Promise`\<`number`\>

Defined in: [index.ts:2518](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2518)

#### Returns

`Promise`\<`number`\>

***

### seriesLabel()

> **seriesLabel**(`seriesIndex`): `Promise`\<`string` \| `null`\>

Defined in: [index.ts:2531](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2531)

#### Parameters

##### seriesIndex

`number`

#### Returns

`Promise`\<`string` \| `null`\>

***

### seriesVisible()

> **seriesVisible**(`seriesIndex`): `Promise`\<`boolean`\>

Defined in: [index.ts:2544](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2544)

#### Parameters

##### seriesIndex

`number`

#### Returns

`Promise`\<`boolean`\>

***

### setBackendPreference()

> **setBackendPreference**(`backendPreference`): `void`

Defined in: [index.ts:2384](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2384)

#### Parameters

##### backendPreference

[`BackendPreference`](../../shared/type-aliases/BackendPreference.md)

#### Returns

`void`

***

### setPlot()

> **setPlot**(`plot`): `Promise`\<`void`\>

Defined in: [index.ts:2326](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2326)

#### Parameters

##### plot

[`PlotBuilder`](PlotBuilder.md)

#### Returns

`Promise`\<`void`\>

***

### setSeriesVisible()

> **setSeriesVisible**(`seriesIndex`, `visible`): `Promise`\<`boolean`\>

Defined in: [index.ts:2562](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2562)

Show or hide a series and re-render; the legend entry stays, dimmed.
A grouped series toggles with its group. Resolves `false` when the
index is out of range or no plot is attached.

#### Parameters

##### seriesIndex

`number`

##### visible

`boolean`

#### Returns

`Promise`\<`boolean`\>

***

### setTime()

> **setTime**(`timeSeconds`): `void`

Defined in: [index.ts:2371](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2371)

#### Parameters

##### timeSeconds

`number`

#### Returns

`void`

***

### wheel()

> **wheel**(`deltaY`, `x`, `y`): `void`

Defined in: [index.ts:2473](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2473)

#### Parameters

##### deltaY

`number`

##### x

`number`

##### y

`number`

#### Returns

`void`
