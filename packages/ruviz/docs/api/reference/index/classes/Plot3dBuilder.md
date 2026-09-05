[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / Plot3dBuilder

# Class: Plot3dBuilder

Defined in: [3d.ts:563](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L563)

Fluent high-level WebGPU 3D plot builder.

Series methods append in drawing order, matching the Rust builders.
Use clearSeries() before adding a replacement.

## Constructors

### Constructor

> **new Plot3dBuilder**(): `Plot3dBuilder`

#### Returns

`Plot3dBuilder`

## Methods

### axisAspect()

> **axisAspect**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:618](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L618)

Fix the plotting box's X:Y:Z proportions; each component must be positive.

#### Parameters

##### x

`number`

##### y

`number`

##### z

`number`

#### Returns

`this`

***

### clearSeries()

> **clearSeries**(): `this`

Defined in: [3d.ts:612](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L612)

Remove series while retaining plot options, for explicit replacement.

#### Returns

`this`

***

### equalScale()

> **equalScale**(): `this`

Defined in: [3d.ts:638](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L638)

Keep equal data units equally long on all axes, preserving physical shapes.

#### Returns

`this`

***

### line3d()

> **line3d**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:578](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L578)

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

Defined in: [3d.ts:654](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L654)

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

Defined in: [3d.ts:569](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L569)

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

### stableScale()

> **stableScale**(`enabled?`): `this`

Defined in: [3d.ts:644](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L644)

Keep framing and scale fixed while rotating; explicit zoom still works.

#### Parameters

##### enabled?

`boolean` = `true`

#### Returns

`this`

***

### surface()

> **surface**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:587](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L587)

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

Defined in: [3d.ts:649](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L649)

#### Parameters

##### title

`string`

#### Returns

`this`

***

### wireframe()

> **wireframe**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:599](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L599)

#### Parameters

##### x

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### y

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### z

[`GridValues3d`](../type-aliases/GridValues3d.md)

#### Returns

`this`
