[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / WorkerSession

# Class: WorkerSession

Defined in: [index.ts:1487](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1487)

Worker-backed interactive canvas session with main-thread fallback support.

## Constructors

### Constructor

> **new WorkerSession**(`canvas`, `mode`, `fallbackSession?`): `WorkerSession`

Defined in: [index.ts:1502](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1502)

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

Defined in: [index.ts:1488](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1488)

## Accessors

### canvas

#### Get Signature

> **get** **canvas**(): `HTMLCanvasElement`

Defined in: [index.ts:1524](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1524)

##### Returns

`HTMLCanvasElement`

## Methods

### \_pushCleanup()

> **\_pushCleanup**(`dispose`): `void`

Defined in: [index.ts:1759](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1759)

#### Parameters

##### dispose

() => `void`

#### Returns

`void`

***

### attachWorker()

> **attachWorker**(`worker`): `void`

Defined in: [index.ts:1528](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1528)

#### Parameters

##### worker

`Worker`

#### Returns

`void`

***

### destroy()

> **destroy**(): `void`

Defined in: [index.ts:1728](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1728)

#### Returns

`void`

***

### dispose()

> **dispose**(): `void`

Defined in: [index.ts:1740](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1740)

#### Returns

`void`

***

### exportPng()

> **exportPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [index.ts:1698](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1698)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### exportSvg()

> **exportSvg**(): `Promise`\<`string`\>

Defined in: [index.ts:1711](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1711)

#### Returns

`Promise`\<`string`\>

***

### hasPlot()

> **hasPlot**(): `boolean`

Defined in: [index.ts:1539](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1539)

#### Returns

`boolean`

***

### pointerDown()

> **pointerDown**(`x`, `y`, `button`): `void`

Defined in: [index.ts:1633](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1633)

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

Defined in: [index.ts:1672](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1672)

#### Returns

`void`

***

### pointerMove()

> **pointerMove**(`x`, `y`): `void`

Defined in: [index.ts:1646](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1646)

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

Defined in: [index.ts:1659](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1659)

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

Defined in: [index.ts:1724](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1724)

#### Returns

`Promise`\<`void`\>

***

### render()

> **render**(): `void`

Defined in: [index.ts:1607](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1607)

#### Returns

`void`

***

### resetView()

> **resetView**(): `void`

Defined in: [index.ts:1620](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1620)

#### Returns

`void`

***

### resize()

> **resize**(`width?`, `height?`, `scaleFactor?`): `void`

Defined in: [index.ts:1569](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1569)

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

Defined in: [index.ts:1596](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1596)

#### Parameters

##### backendPreference

[`BackendPreference`](../../shared/type-aliases/BackendPreference.md)

#### Returns

`void`

***

### setPlot()

> **setPlot**(`plot`): `Promise`\<`void`\>

Defined in: [index.ts:1547](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1547)

#### Parameters

##### plot

[`PlotBuilder`](PlotBuilder.md)

#### Returns

`Promise`\<`void`\>

***

### setTime()

> **setTime**(`timeSeconds`): `void`

Defined in: [index.ts:1583](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1583)

#### Parameters

##### timeSeconds

`number`

#### Returns

`void`

***

### wheel()

> **wheel**(`deltaY`, `x`, `y`): `void`

Defined in: [index.ts:1685](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1685)

#### Parameters

##### deltaY

`number`

##### x

`number`

##### y

`number`

#### Returns

`void`
