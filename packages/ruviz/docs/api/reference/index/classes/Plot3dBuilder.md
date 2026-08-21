[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / Plot3dBuilder

# Class: Plot3dBuilder

Defined in: [3d.ts:506](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L506)

Fluent high-level WebGPU 3D plot builder.

A builder describes one 3D series. Calling another series method replaces
the previous series, matching the raw browser bridge.

## Constructors

### Constructor

> **new Plot3dBuilder**(): `Plot3dBuilder`

#### Returns

`Plot3dBuilder`

## Methods

### line3d()

> **line3d**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:519](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L519)

#### Parameters

##### x

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### y

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### z

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`this`

***

### mount()

> **mount**(`canvas`, `options?`): `Promise`\<[`Plot3dSession`](../interfaces/Plot3dSession.md)\>

Defined in: [3d.ts:557](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L557)

#### Parameters

##### canvas

`HTMLCanvasElement` \| `OffscreenCanvas`

##### options?

[`Plot3dMountOptions`](../interfaces/Plot3dMountOptions.md) = `{}`

#### Returns

`Promise`\<[`Plot3dSession`](../interfaces/Plot3dSession.md)\>

***

### scatter3d()

> **scatter3d**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:510](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L510)

#### Parameters

##### x

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### y

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### z

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`this`

***

### surface()

> **surface**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:528](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L528)

#### Parameters

##### x

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### y

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### z

[`GridValues3d`](../type-aliases/GridValues3d.md)

#### Returns

`this`

***

### title()

> **title**(`title`): `this`

Defined in: [3d.ts:552](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L552)

#### Parameters

##### title

`string`

#### Returns

`this`

***

### wireframe()

> **wireframe**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:540](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L540)

#### Parameters

##### x

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### y

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### z

[`GridValues3d`](../type-aliases/GridValues3d.md)

#### Returns

`this`
