[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / CanvasSession

# Class: CanvasSession

Defined in: [index.ts:1341](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1341)

Main-thread interactive canvas session.

## Constructors

### Constructor

> **new CanvasSession**(`module`, `rawSession`, `canvas`): `CanvasSession`

Defined in: [index.ts:1349](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1349)

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

Defined in: [index.ts:1342](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1342)

## Methods

### \_pushCleanup()

> **\_pushCleanup**(`dispose`): `void`

Defined in: [index.ts:1469](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1469)

#### Parameters

##### dispose

() => `void`

#### Returns

`void`

***

### destroy()

> **destroy**(): `void`

Defined in: [index.ts:1458](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1458)

#### Returns

`void`

***

### dispose()

> **dispose**(): `void`

Defined in: [index.ts:1462](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1462)

#### Returns

`void`

***

### exportPng()

> **exportPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [index.ts:1442](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1442)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### exportSvg()

> **exportSvg**(): `Promise`\<`string`\>

Defined in: [index.ts:1450](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1450)

#### Returns

`Promise`\<`string`\>

***

### hasPlot()

> **hasPlot**(): `boolean`

Defined in: [index.ts:1356](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1356)

#### Returns

`boolean`

***

### pointerDown()

> **pointerDown**(`x`, `y`, `button`): `void`

Defined in: [index.ts:1412](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1412)

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

Defined in: [index.ts:1430](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1430)

#### Returns

`void`

***

### pointerMove()

> **pointerMove**(`x`, `y`): `void`

Defined in: [index.ts:1418](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1418)

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

Defined in: [index.ts:1424](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1424)

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

Defined in: [index.ts:1400](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1400)

#### Returns

`void`

***

### resetView()

> **resetView**(): `void`

Defined in: [index.ts:1406](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1406)

#### Returns

`void`

***

### resize()

> **resize**(`width?`, `height?`, `scaleFactor?`): `void`

Defined in: [index.ts:1381](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1381)

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

Defined in: [index.ts:1394](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1394)

#### Parameters

##### backendPreference

[`BackendPreference`](../../shared/type-aliases/BackendPreference.md)

#### Returns

`void`

***

### setPlot()

> **setPlot**(`plot`): `Promise`\<`void`\>

Defined in: [index.ts:1376](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1376)

#### Parameters

##### plot

[`PlotBuilder`](PlotBuilder.md)

#### Returns

`Promise`\<`void`\>

***

### setTime()

> **setTime**(`timeSeconds`): `void`

Defined in: [index.ts:1390](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1390)

#### Parameters

##### timeSeconds

`number`

#### Returns

`void`

***

### wheel()

> **wheel**(`deltaY`, `x`, `y`): `void`

Defined in: [index.ts:1436](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1436)

#### Parameters

##### deltaY

`number`

##### x

`number`

##### y

`number`

#### Returns

`void`
