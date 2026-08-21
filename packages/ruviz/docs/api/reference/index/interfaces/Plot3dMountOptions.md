[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / Plot3dMountOptions

# Interface: Plot3dMountOptions

Defined in: [3d.ts:29](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L29)

## Properties

### autoResize?

> `optional` **autoResize?**: `boolean`

Defined in: [3d.ts:36](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L36)

Keep an HTML canvas backing surface synchronized with its CSS size.

Defaults to `true` for `HTMLCanvasElement` and is unavailable for an
`OffscreenCanvas`.

***

### bindInput?

> `optional` **bindInput?**: `boolean`

Defined in: [3d.ts:44](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L44)

Bind orbit, pan, zoom, reset, and picking input to an HTML canvas.

Defaults to `true` for `HTMLCanvasElement` and is unavailable for an
`OffscreenCanvas`.

***

### scaleFactor?

> `optional` **scaleFactor?**: `number`

Defined in: [3d.ts:53](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/3d.ts#L53)

Device scale used for text and point sizing.

Defaults to `window.devicePixelRatio` on the main thread and `1` for an
`OffscreenCanvas`. Pass the main thread's device pixel ratio when mounting
a transferred canvas in a worker.
