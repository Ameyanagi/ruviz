[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / WorkerSession

# Class: WorkerSession

Defined in: [index.ts:2269](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2269)

Worker-backed interactive canvas session with main-thread fallback support.

## Constructors

### Constructor

> **new WorkerSession**(`canvas`, `mode`, `fallbackSession?`): `WorkerSession`

Defined in: [index.ts:2286](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2286)

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

Defined in: [index.ts:2270](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2270)

## Accessors

### canvas

#### Get Signature

> **get** **canvas**(): `HTMLCanvasElement`

Defined in: [index.ts:2310](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2310)

##### Returns

`HTMLCanvasElement`

## Methods

### \_pushCleanup()

> **\_pushCleanup**(`dispose`): `void`

Defined in: [index.ts:2658](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2658)

#### Parameters

##### dispose

() => `void`

#### Returns

`void`

***

### attachWorker()

> **attachWorker**(`worker`): `void`

Defined in: [index.ts:2314](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2314)

#### Parameters

##### worker

`Worker`

#### Returns

`void`

***

### ~~destroy()~~

> **destroy**(): `void`

Defined in: [index.ts:2630](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2630)

#### Returns

`void`

#### Deprecated

Use detach() to clear the plot, or dispose() to remove bindings.

***

### detach()

> **detach**(): `void`

Defined in: [index.ts:2613](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2613)

Remove the plot but keep input/resize bindings for a later setPlot().

#### Returns

`void`

***

### dispose()

> **dispose**(): `void`

Defined in: [index.ts:2635](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2635)

Remove event/resize bindings and release the attached plot and worker.

#### Returns

`void`

***

### exportPng()

> **exportPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [index.ts:2582](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2582)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### exportSvg()

> **exportSvg**(): `Promise`\<`string`\>

Defined in: [index.ts:2595](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2595)

#### Returns

`Promise`\<`string`\>

***

### hasPlot()

> **hasPlot**(): `boolean`

Defined in: [index.ts:2325](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2325)

#### Returns

`boolean`

***

### hitAt()

> **hitAt**(`x`, `y`): `Promise`\<[`SeriesHit`](../../shared/interfaces/SeriesHit.md) \| `null`\>

Defined in: [index.ts:2498](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2498)

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

Defined in: [index.ts:2512](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2512)

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

Defined in: [index.ts:2428](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2428)

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

Defined in: [index.ts:2467](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2467)

#### Returns

`void`

***

### pointerMove()

> **pointerMove**(`x`, `y`): `void`

Defined in: [index.ts:2441](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2441)

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

Defined in: [index.ts:2454](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2454)

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

Defined in: [index.ts:2608](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2608)

#### Returns

`Promise`\<`void`\>

***

### render()

> **render**(): `void`

Defined in: [index.ts:2402](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2402)

#### Returns

`void`

***

### resetView()

> **resetView**(): `void`

Defined in: [index.ts:2415](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2415)

#### Returns

`void`

***

### resize()

> **resize**(`width?`, `height?`, `scaleFactor?`): `void`

Defined in: [index.ts:2364](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2364)

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

Defined in: [index.ts:2525](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2525)

#### Returns

`Promise`\<`number`\>

***

### seriesLabel()

> **seriesLabel**(`seriesIndex`): `Promise`\<`string` \| `null`\>

Defined in: [index.ts:2538](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2538)

#### Parameters

##### seriesIndex

`number`

#### Returns

`Promise`\<`string` \| `null`\>

***

### seriesVisible()

> **seriesVisible**(`seriesIndex`): `Promise`\<`boolean`\>

Defined in: [index.ts:2551](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2551)

#### Parameters

##### seriesIndex

`number`

#### Returns

`Promise`\<`boolean`\>

***

### setBackendPreference()

> **setBackendPreference**(`backendPreference`): `void`

Defined in: [index.ts:2391](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2391)

#### Parameters

##### backendPreference

[`BackendPreference`](../../shared/type-aliases/BackendPreference.md)

#### Returns

`void`

***

### setPlot()

> **setPlot**(`plot`): `Promise`\<`void`\>

Defined in: [index.ts:2333](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2333)

#### Parameters

##### plot

[`PlotBuilder`](PlotBuilder.md)

#### Returns

`Promise`\<`void`\>

***

### setSeriesVisible()

> **setSeriesVisible**(`seriesIndex`, `visible`): `Promise`\<`boolean`\>

Defined in: [index.ts:2569](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2569)

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

Defined in: [index.ts:2378](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2378)

#### Parameters

##### timeSeconds

`number`

#### Returns

`void`

***

### wheel()

> **wheel**(`deltaY`, `x`, `y`): `void`

Defined in: [index.ts:2480](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2480)

#### Parameters

##### deltaY

`number`

##### x

`number`

##### y

`number`

#### Returns

`void`
