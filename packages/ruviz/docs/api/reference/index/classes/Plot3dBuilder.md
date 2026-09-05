[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / Plot3dBuilder

# Class: Plot3dBuilder

Defined in: [3d.ts:567](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L567)

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

Defined in: [3d.ts:622](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L622)

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

Defined in: [3d.ts:616](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L616)

Remove series while retaining plot options, for explicit replacement.

#### Returns

`this`

***

### equalScale()

> **equalScale**(): `this`

Defined in: [3d.ts:642](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L642)

Keep equal data units equally long on all axes, preserving physical shapes.

#### Returns

`this`

***

### line3d()

> **line3d**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:582](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L582)

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

Defined in: [3d.ts:658](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L658)

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

Defined in: [3d.ts:573](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L573)

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

Defined in: [3d.ts:648](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L648)

Keep framing and scale fixed while rotating; explicit zoom still works.

#### Parameters

##### enabled?

`boolean` = `true`

#### Returns

`this`

***

### surface()

> **surface**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:591](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L591)

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

Defined in: [3d.ts:653](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L653)

#### Parameters

##### title

`string`

#### Returns

`this`

***

### wireframe()

> **wireframe**(`x`, `y`, `z`): `this`

Defined in: [3d.ts:603](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L603)

#### Parameters

##### x

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### y

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### z

[`GridValues3d`](../type-aliases/GridValues3d.md)

#### Returns

`this`
