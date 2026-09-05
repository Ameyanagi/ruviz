[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / Plot3dSession

# Interface: Plot3dSession

Defined in: [3d.ts:67](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L67)

A mounted retained WebGPU 3D plot.

Input, resize, and `render()` calls are coalesced into at most one WebGPU
submission per animation frame.

## Properties

### canvas

> `readonly` **canvas**: `HTMLCanvasElement` \| `OffscreenCanvas`

Defined in: [3d.ts:69](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L69)

***

### error

> `readonly` **error**: `Error` \| `null`

Defined in: [3d.ts:75](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L75)

Last scheduled failure, cleared after a successful render.

***

### mode

> `readonly` **mode**: [`Plot3dSessionMode`](../type-aliases/Plot3dSessionMode.md)

Defined in: [3d.ts:68](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L68)

## Methods

### backend()

> **backend**(): `string`

Defined in: [3d.ts:87](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L87)

#### Returns

`string`

***

### destroy()

> **destroy**(): `void`

Defined in: [3d.ts:91](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L91)

#### Returns

`void`

***

### dispose()

> **dispose**(): `void`

Defined in: [3d.ts:92](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L92)

#### Returns

`void`

***

### doubleClick()

> **doubleClick**(`x`, `y`): `void`

Defined in: [3d.ts:82](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L82)

#### Parameters

##### x

`number`

##### y

`number`

#### Returns

`void`

***

### exportPng()

> **exportPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [3d.ts:89](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L89)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### needsRecreate()

> **needsRecreate**(): `boolean`

Defined in: [3d.ts:88](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L88)

#### Returns

`boolean`

***

### pointerDown()

> **pointerDown**(`x`, `y`, `button`): `void`

Defined in: [3d.ts:79](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L79)

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

### pointerMove()

> **pointerMove**(`x`, `y`): `void`

Defined in: [3d.ts:80](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L80)

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

Defined in: [3d.ts:81](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L81)

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

Defined in: [3d.ts:72](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L72)

Request a frame. Multiple requests in one animation frame are coalesced.

#### Returns

`void`

***

### resetView()

> **resetView**(): `void`

Defined in: [3d.ts:78](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L78)

#### Returns

`void`

***

### resize()

> **resize**(`width?`, `height?`, `scaleFactor?`): `void`

Defined in: [3d.ts:77](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L77)

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

### selectedSeries()

> **selectedSeries**(): `number` \| `null`

Defined in: [3d.ts:85](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L85)

#### Returns

`number` \| `null`

***

### selectedSource()

> **selectedSource**(): `number` \| `null`

Defined in: [3d.ts:86](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L86)

#### Returns

`number` \| `null`

***

### wheel()

> **wheel**(`deltaY`): `void`

Defined in: [3d.ts:83](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L83)

#### Parameters

##### deltaY

`number`

#### Returns

`void`
