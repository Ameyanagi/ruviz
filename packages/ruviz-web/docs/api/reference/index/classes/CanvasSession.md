[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / CanvasSession

# Class: CanvasSession

Defined in: [index.ts:1353](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1353)

Main-thread interactive canvas session.

## Constructors

### Constructor

> **new CanvasSession**(`module`, `rawSession`, `canvas`): `CanvasSession`

Defined in: [index.ts:1361](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1361)

#### Parameters

##### module

`__module`

##### rawSession

`WebCanvasSession`

##### canvas

`HTMLCanvasElement`

#### Returns

`CanvasSession`

## Properties

### mode

> `readonly` **mode**: [`SessionMode`](../../shared/type-aliases/SessionMode.md) = `"main-thread"`

Defined in: [index.ts:1354](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1354)

## Methods

### \_pushCleanup()

> **\_pushCleanup**(`dispose`): `void`

Defined in: [index.ts:1481](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1481)

#### Parameters

##### dispose

() => `void`

#### Returns

`void`

***

### destroy()

> **destroy**(): `void`

Defined in: [index.ts:1470](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1470)

#### Returns

`void`

***

### dispose()

> **dispose**(): `void`

Defined in: [index.ts:1474](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1474)

#### Returns

`void`

***

### exportPng()

> **exportPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [index.ts:1454](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1454)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### exportSvg()

> **exportSvg**(): `Promise`\<`string`\>

Defined in: [index.ts:1462](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1462)

#### Returns

`Promise`\<`string`\>

***

### hasPlot()

> **hasPlot**(): `boolean`

Defined in: [index.ts:1368](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1368)

#### Returns

`boolean`

***

### pointerDown()

> **pointerDown**(`x`, `y`, `button`): `void`

Defined in: [index.ts:1424](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1424)

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

Defined in: [index.ts:1442](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1442)

#### Returns

`void`

***

### pointerMove()

> **pointerMove**(`x`, `y`): `void`

Defined in: [index.ts:1430](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1430)

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

Defined in: [index.ts:1436](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1436)

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

### render()

> **render**(): `void`

Defined in: [index.ts:1412](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1412)

#### Returns

`void`

***

### resetView()

> **resetView**(): `void`

Defined in: [index.ts:1418](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1418)

#### Returns

`void`

***

### resize()

> **resize**(`width?`, `height?`, `scaleFactor?`): `void`

Defined in: [index.ts:1393](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1393)

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

Defined in: [index.ts:1406](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1406)

#### Parameters

##### backendPreference

[`BackendPreference`](../../shared/type-aliases/BackendPreference.md)

#### Returns

`void`

***

### setPlot()

> **setPlot**(`plot`): `Promise`\<`void`\>

Defined in: [index.ts:1388](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1388)

#### Parameters

##### plot

[`PlotBuilder`](PlotBuilder.md)

#### Returns

`Promise`\<`void`\>

***

### setTime()

> **setTime**(`timeSeconds`): `void`

Defined in: [index.ts:1402](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1402)

#### Parameters

##### timeSeconds

`number`

#### Returns

`void`

***

### wheel()

> **wheel**(`deltaY`, `x`, `y`): `void`

Defined in: [index.ts:1448](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1448)

#### Parameters

##### deltaY

`number`

##### x

`number`

##### y

`number`

#### Returns

`void`
