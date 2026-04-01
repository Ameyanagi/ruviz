[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / PlotBuilder

# Class: PlotBuilder

Defined in: [index.ts:837](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L837)

Fluent plot builder for static export and interactive canvas mounting.

## Constructors

### Constructor

> **new PlotBuilder**(`state?`): `PlotBuilder`

Defined in: [index.ts:840](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L840)

#### Parameters

##### state?

`PlotState`

#### Returns

`PlotBuilder`

## Methods

### \_toRawPlot()

> **\_toRawPlot**(`module?`): `Promise`\<`JsPlot`\>

Defined in: [index.ts:1272](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1272)

#### Parameters

##### module?

`__module`

#### Returns

`Promise`\<`JsPlot`\>

***

### addLine()

> **addLine**(`input`): `this`

Defined in: [index.ts:1069](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1069)

#### Parameters

##### input

`XYSeriesInput`

#### Returns

`this`

***

### addScatter()

> **addScatter**(`input`): `this`

Defined in: [index.ts:1077](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1077)

#### Parameters

##### input

`XYSeriesInput`

#### Returns

`this`

***

### bar()

> **bar**(`input`): `this`

Defined in: [index.ts:1081](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1081)

#### Parameters

##### input

`BarSeriesInput`

#### Returns

`this`

***

### boxplot()

> **boxplot**(`input`): `this`

Defined in: [index.ts:1097](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1097)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md) \| [`ObservableSeries`](ObservableSeries.md)

#### Returns

`this`

***

### clone()

> **clone**(): `PlotBuilder`

Defined in: [index.ts:1208](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1208)

#### Returns

`PlotBuilder`

***

### contour()

> **contour**(`input`): `this`

Defined in: [index.ts:1150](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1150)

#### Parameters

##### input

`ContourInput`

#### Returns

`this`

***

### ecdf()

> **ecdf**(`input`): `this`

Defined in: [index.ts:1145](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1145)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`this`

***

### errorBars()

> **errorBars**(`input`): `this`

Defined in: [index.ts:1109](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1109)

#### Parameters

##### input

`ErrorBarsInput`

#### Returns

`this`

***

### errorBarsXY()

> **errorBarsXY**(`input`): `this`

Defined in: [index.ts:1123](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1123)

#### Parameters

##### input

`ErrorBarsXYInput`

#### Returns

`this`

***

### heatmap()

> **heatmap**(`input`): `this`

Defined in: [index.ts:1103](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1103)

#### Parameters

##### input

readonly [`NumericArray`](../../shared/type-aliases/NumericArray.md)[]

#### Returns

`this`

***

### histogram()

> **histogram**(`input`): `this`

Defined in: [index.ts:1091](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1091)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md) \| [`ObservableSeries`](ObservableSeries.md)

#### Returns

`this`

***

### kde()

> **kde**(`input`): `this`

Defined in: [index.ts:1140](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1140)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`this`

***

### line()

> **line**(`input`): `this`

Defined in: [index.ts:1065](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1065)

#### Parameters

##### input

`XYSeriesInput`

#### Returns

`this`

***

### mount()

> **mount**(`canvas`, `options?`): `Promise`\<[`CanvasSession`](CanvasSession.md)\>

Defined in: [index.ts:1252](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1252)

#### Parameters

##### canvas

`HTMLCanvasElement`

##### options?

[`CanvasSessionOptions`](../../shared/interfaces/CanvasSessionOptions.md)

#### Returns

`Promise`\<[`CanvasSession`](CanvasSession.md)\>

***

### mountWorker()

> **mountWorker**(`canvas`, `options?`): `Promise`\<[`WorkerSession`](WorkerSession.md)\>

Defined in: [index.ts:1262](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1262)

#### Parameters

##### canvas

`HTMLCanvasElement`

##### options?

[`WorkerSessionOptions`](../../shared/interfaces/WorkerSessionOptions.md)

#### Returns

`Promise`\<[`WorkerSession`](WorkerSession.md)\>

***

### pie()

> **pie**(`values`, `labelsInput?`): `this`

Defined in: [index.ts:1161](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1161)

#### Parameters

##### values

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### labelsInput?

`ArrayLike`\<`string`\> \| readonly `string`[]

#### Returns

`this`

***

### polarLine()

> **polarLine**(`input`): `this`

Defined in: [index.ts:1198](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1198)

#### Parameters

##### input

`PolarLineInput`

#### Returns

`this`

***

### radar()

> **radar**(`input`): `this`

Defined in: [index.ts:1171](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1171)

#### Parameters

##### input

`RadarInput`

#### Returns

`this`

***

### renderPng()

> **renderPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [index.ts:1226](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1226)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### renderSvg()

> **renderSvg**(): `Promise`\<`string`\>

Defined in: [index.ts:1232](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1232)

#### Returns

`Promise`\<`string`\>

***

### save()

> **save**(`options?`): `Promise`\<`void`\>

Defined in: [index.ts:1238](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1238)

#### Parameters

##### options?

[`PlotSaveOptions`](../../shared/interfaces/PlotSaveOptions.md)

#### Returns

`Promise`\<`void`\>

***

### scatter()

> **scatter**(`input`): `this`

Defined in: [index.ts:1073](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1073)

#### Parameters

##### input

`XYSeriesInput`

#### Returns

`this`

***

### setSizePx()

> **setSizePx**(`width`, `height`): `this`

Defined in: [index.ts:1015](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1015)

#### Parameters

##### width

`number`

##### height

`number`

#### Returns

`this`

***

### setTheme()

> **setTheme**(`theme`): `this`

Defined in: [index.ts:1024](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1024)

#### Parameters

##### theme

[`PlotTheme`](../../shared/type-aliases/PlotTheme.md)

#### Returns

`this`

***

### setTicks()

> **setTicks**(`enabled`): `this`

Defined in: [index.ts:1033](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1033)

#### Parameters

##### enabled

`boolean`

#### Returns

`this`

***

### setTitle()

> **setTitle**(`title`): `this`

Defined in: [index.ts:1042](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1042)

#### Parameters

##### title

`string`

#### Returns

`this`

***

### setXLabel()

> **setXLabel**(`label`): `this`

Defined in: [index.ts:1051](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1051)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### setYLabel()

> **setYLabel**(`label`): `this`

Defined in: [index.ts:1060](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1060)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### sizePx()

> **sizePx**(`width`, `height`): `this`

Defined in: [index.ts:1011](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1011)

#### Parameters

##### width

`number`

##### height

`number`

#### Returns

`this`

***

### theme()

> **theme**(`theme`): `this`

Defined in: [index.ts:1020](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1020)

#### Parameters

##### theme

[`PlotTheme`](../../shared/type-aliases/PlotTheme.md)

#### Returns

`this`

***

### ticks()

> **ticks**(`enabled`): `this`

Defined in: [index.ts:1029](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1029)

#### Parameters

##### enabled

`boolean`

#### Returns

`this`

***

### title()

> **title**(`title`): `this`

Defined in: [index.ts:1038](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1038)

#### Parameters

##### title

`string`

#### Returns

`this`

***

### toSnapshot()

> **toSnapshot**(): [`PlotSnapshot`](../../shared/interfaces/PlotSnapshot.md)

Defined in: [index.ts:1212](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1212)

#### Returns

[`PlotSnapshot`](../../shared/interfaces/PlotSnapshot.md)

***

### violin()

> **violin**(`input`): `this`

Defined in: [index.ts:1193](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1193)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`this`

***

### xlabel()

> **xlabel**(`label`): `this`

Defined in: [index.ts:1047](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1047)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### ylabel()

> **ylabel**(`label`): `this`

Defined in: [index.ts:1056](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L1056)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### fromSnapshot()

> `static` **fromSnapshot**(`snapshot`): `PlotBuilder`

Defined in: [index.ts:854](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L854)

#### Parameters

##### snapshot

[`PlotSnapshot`](../../shared/interfaces/PlotSnapshot.md)

#### Returns

`PlotBuilder`
