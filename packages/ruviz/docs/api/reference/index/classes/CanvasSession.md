[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / CanvasSession

# Class: CanvasSession

Defined in: [index.ts:2063](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2063)

Main-thread interactive canvas session.

## Constructors

### Constructor

> **new CanvasSession**(`module`, `rawSession`, `canvas`): `CanvasSession`

Defined in: [index.ts:2073](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2073)

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

Defined in: [index.ts:2064](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2064)

## Methods

### \_pushCleanup()

> **\_pushCleanup**(`dispose`): `void`

Defined in: [index.ts:2256](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2256)

#### Parameters

##### dispose

() => `void`

#### Returns

`void`

***

### destroy()

> **destroy**(): `void`

Defined in: [index.ts:2241](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2241)

#### Returns

`void`

***

### dispose()

> **dispose**(): `void`

Defined in: [index.ts:2247](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2247)

#### Returns

`void`

***

### exportPng()

> **exportPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [index.ts:2225](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2225)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### exportSvg()

> **exportSvg**(): `Promise`\<`string`\>

Defined in: [index.ts:2233](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2233)

#### Returns

`Promise`\<`string`\>

***

### hasPlot()

> **hasPlot**(): `boolean`

Defined in: [index.ts:2082](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2082)

#### Returns

`boolean`

***

### hitAt()

> **hitAt**(`x`, `y`): [`SeriesHit`](../../shared/interfaces/SeriesHit.md) \| `null`

Defined in: [index.ts:2175](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2175)

The series data point under a canvas pixel, or `null`.

#### Parameters

##### x

`number`

##### y

`number`

#### Returns

[`SeriesHit`](../../shared/interfaces/SeriesHit.md) \| `null`

***

### legendEntryAt()

> **legendEntryAt**(`x`, `y`): `number` \| `null`

Defined in: [index.ts:2195](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2195)

The index of the series behind a legend entry at a canvas pixel, or `null`.

#### Parameters

##### x

`number`

##### y

`number`

#### Returns

`number` \| `null`

***

### pointerDown()

> **pointerDown**(`x`, `y`, `button`): `void`

Defined in: [index.ts:2144](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2144)

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

Defined in: [index.ts:2162](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2162)

#### Returns

`void`

***

### pointerMove()

> **pointerMove**(`x`, `y`): `void`

Defined in: [index.ts:2150](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2150)

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

Defined in: [index.ts:2156](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2156)

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

Defined in: [index.ts:2132](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2132)

#### Returns

`void`

***

### resetView()

> **resetView**(): `void`

Defined in: [index.ts:2138](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2138)

#### Returns

`void`

***

### resize()

> **resize**(`width?`, `height?`, `scaleFactor?`): `void`

Defined in: [index.ts:2113](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2113)

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

> **seriesCount**(): `number`

Defined in: [index.ts:2200](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2200)

#### Returns

`number`

***

### seriesLabel()

> **seriesLabel**(`seriesIndex`): `string` \| `null`

Defined in: [index.ts:2204](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2204)

#### Parameters

##### seriesIndex

`number`

#### Returns

`string` \| `null`

***

### seriesVisible()

> **seriesVisible**(`seriesIndex`): `boolean`

Defined in: [index.ts:2208](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2208)

#### Parameters

##### seriesIndex

`number`

#### Returns

`boolean`

***

### setBackendPreference()

> **setBackendPreference**(`backendPreference`): `void`

Defined in: [index.ts:2126](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2126)

#### Parameters

##### backendPreference

[`BackendPreference`](../../shared/type-aliases/BackendPreference.md)

#### Returns

`void`

***

### setPlot()

> **setPlot**(`plot`): `Promise`\<`void`\>

Defined in: [index.ts:2102](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2102)

#### Parameters

##### plot

[`PlotBuilder`](PlotBuilder.md)

#### Returns

`Promise`\<`void`\>

***

### setSeriesVisible()

> **setSeriesVisible**(`seriesIndex`, `visible`): `boolean`

Defined in: [index.ts:2217](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2217)

Show or hide a series and re-render; the legend entry stays, dimmed.
A grouped series toggles with its group. Returns `false` when the index
is out of range or no plot is attached.

#### Parameters

##### seriesIndex

`number`

##### visible

`boolean`

#### Returns

`boolean`

***

### setTime()

> **setTime**(`timeSeconds`): `void`

Defined in: [index.ts:2122](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2122)

#### Parameters

##### timeSeconds

`number`

#### Returns

`void`

***

### wheel()

> **wheel**(`deltaY`, `x`, `y`): `void`

Defined in: [index.ts:2168](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L2168)

#### Parameters

##### deltaY

`number`

##### x

`number`

##### y

`number`

#### Returns

`void`
