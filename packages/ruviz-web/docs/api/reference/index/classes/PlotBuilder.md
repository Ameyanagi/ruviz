[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / PlotBuilder

# Class: PlotBuilder

Defined in: [index.ts:819](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L819)

Fluent plot builder for static export and interactive canvas mounting.

## Constructors

### Constructor

> **new PlotBuilder**(`state?`): `PlotBuilder`

Defined in: [index.ts:822](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L822)

#### Parameters

##### state?

`PlotState`

#### Returns

`PlotBuilder`

## Methods

### \_toRawPlot()

> **\_toRawPlot**(`module?`): `Promise`\<`JsPlot`\>

Defined in: [index.ts:1260](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1260)

#### Parameters

##### module?

`__module`

#### Returns

`Promise`\<`JsPlot`\>

***

### addLine()

> **addLine**(`input`): `this`

Defined in: [index.ts:1063](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1063)

#### Parameters

##### input

`XYSeriesInput`

#### Returns

`this`

***

### addScatter()

> **addScatter**(`input`): `this`

Defined in: [index.ts:1071](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1071)

#### Parameters

##### input

`XYSeriesInput`

#### Returns

`this`

***

### bar()

> **bar**(`input`): `this`

Defined in: [index.ts:1075](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1075)

#### Parameters

##### input

`BarSeriesInput`

#### Returns

`this`

***

### boxplot()

> **boxplot**(`input`): `this`

Defined in: [index.ts:1091](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1091)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md) \| [`ObservableSeries`](ObservableSeries.md)

#### Returns

`this`

***

### clone()

> **clone**(): `PlotBuilder`

Defined in: [index.ts:1199](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1199)

#### Returns

`PlotBuilder`

***

### contour()

> **contour**(`input`): `this`

Defined in: [index.ts:1141](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1141)

#### Parameters

##### input

`ContourInput`

#### Returns

`this`

***

### ecdf()

> **ecdf**(`input`): `this`

Defined in: [index.ts:1136](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1136)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`this`

***

### errorBars()

> **errorBars**(`input`): `this`

Defined in: [index.ts:1103](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1103)

#### Parameters

##### input

`ErrorBarsInput`

#### Returns

`this`

***

### errorBarsXY()

> **errorBarsXY**(`input`): `this`

Defined in: [index.ts:1114](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1114)

#### Parameters

##### input

`ErrorBarsXYInput`

#### Returns

`this`

***

### heatmap()

> **heatmap**(`input`): `this`

Defined in: [index.ts:1097](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1097)

#### Parameters

##### input

readonly [`NumericArray`](../../shared/type-aliases/NumericArray.md)[]

#### Returns

`this`

***

### histogram()

> **histogram**(`input`): `this`

Defined in: [index.ts:1085](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1085)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md) \| [`ObservableSeries`](ObservableSeries.md)

#### Returns

`this`

***

### kde()

> **kde**(`input`): `this`

Defined in: [index.ts:1131](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1131)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`this`

***

### line()

> **line**(`input`): `this`

Defined in: [index.ts:1059](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1059)

#### Parameters

##### input

`XYSeriesInput`

#### Returns

`this`

***

### mount()

> **mount**(`canvas`, `options?`): `Promise`\<[`CanvasSession`](CanvasSession.md)\>

Defined in: [index.ts:1243](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1243)

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

Defined in: [index.ts:1250](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1250)

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

Defined in: [index.ts:1152](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1152)

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

Defined in: [index.ts:1189](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1189)

#### Parameters

##### input

`PolarLineInput`

#### Returns

`this`

***

### radar()

> **radar**(`input`): `this`

Defined in: [index.ts:1162](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1162)

#### Parameters

##### input

`RadarInput`

#### Returns

`this`

***

### renderPng()

> **renderPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [index.ts:1217](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1217)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### renderSvg()

> **renderSvg**(): `Promise`\<`string`\>

Defined in: [index.ts:1223](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1223)

#### Returns

`Promise`\<`string`\>

***

### save()

> **save**(`options?`): `Promise`\<`void`\>

Defined in: [index.ts:1229](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1229)

#### Parameters

##### options?

[`PlotSaveOptions`](../../shared/interfaces/PlotSaveOptions.md)

#### Returns

`Promise`\<`void`\>

***

### scatter()

> **scatter**(`input`): `this`

Defined in: [index.ts:1067](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1067)

#### Parameters

##### input

`XYSeriesInput`

#### Returns

`this`

***

### setSizePx()

> **setSizePx**(`width`, `height`): `this`

Defined in: [index.ts:1009](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1009)

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

Defined in: [index.ts:1018](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1018)

#### Parameters

##### theme

[`PlotTheme`](../../shared/type-aliases/PlotTheme.md)

#### Returns

`this`

***

### setTicks()

> **setTicks**(`enabled`): `this`

Defined in: [index.ts:1027](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1027)

#### Parameters

##### enabled

`boolean`

#### Returns

`this`

***

### setTitle()

> **setTitle**(`title`): `this`

Defined in: [index.ts:1036](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1036)

#### Parameters

##### title

`string`

#### Returns

`this`

***

### setXLabel()

> **setXLabel**(`label`): `this`

Defined in: [index.ts:1045](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1045)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### setYLabel()

> **setYLabel**(`label`): `this`

Defined in: [index.ts:1054](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1054)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### sizePx()

> **sizePx**(`width`, `height`): `this`

Defined in: [index.ts:1005](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1005)

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

Defined in: [index.ts:1014](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1014)

#### Parameters

##### theme

[`PlotTheme`](../../shared/type-aliases/PlotTheme.md)

#### Returns

`this`

***

### ticks()

> **ticks**(`enabled`): `this`

Defined in: [index.ts:1023](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1023)

#### Parameters

##### enabled

`boolean`

#### Returns

`this`

***

### title()

> **title**(`title`): `this`

Defined in: [index.ts:1032](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1032)

#### Parameters

##### title

`string`

#### Returns

`this`

***

### toSnapshot()

> **toSnapshot**(): [`PlotSnapshot`](../../shared/interfaces/PlotSnapshot.md)

Defined in: [index.ts:1203](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1203)

#### Returns

[`PlotSnapshot`](../../shared/interfaces/PlotSnapshot.md)

***

### violin()

> **violin**(`input`): `this`

Defined in: [index.ts:1184](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1184)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`this`

***

### xlabel()

> **xlabel**(`label`): `this`

Defined in: [index.ts:1041](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1041)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### ylabel()

> **ylabel**(`label`): `this`

Defined in: [index.ts:1050](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L1050)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### fromSnapshot()

> `static` **fromSnapshot**(`snapshot`): `PlotBuilder`

Defined in: [index.ts:836](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L836)

#### Parameters

##### snapshot

[`PlotSnapshot`](../../shared/interfaces/PlotSnapshot.md)

#### Returns

`PlotBuilder`
