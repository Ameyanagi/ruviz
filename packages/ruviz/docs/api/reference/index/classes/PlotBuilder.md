[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / PlotBuilder

# Class: PlotBuilder

Defined in: [index.ts:1007](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1007)

Fluent plot builder for static export and interactive canvas mounting.

## Constructors

### Constructor

> **new PlotBuilder**(`state?`): `PlotBuilder`

Defined in: [index.ts:1024](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1024)

#### Parameters

##### state?

`PlotState`

#### Returns

`PlotBuilder`

## Methods

### \_revision()

> **\_revision**(): `number`

Defined in: [index.ts:1953](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1953)

#### Returns

`number`

***

### \_toRawPlot()

> **\_toRawPlot**(`module?`): `Promise`\<`JsPlot`\>

Defined in: [index.ts:1918](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1918)

#### Parameters

##### module?

`__module`

#### Returns

`Promise`\<`JsPlot`\>

***

### addLine()

> **addLine**(`input`): `this`

Defined in: [index.ts:1640](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1640)

#### Parameters

##### input

`LineSeriesInput`

#### Returns

`this`

***

### addScatter()

> **addScatter**(`input`): `this`

Defined in: [index.ts:1648](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1648)

#### Parameters

##### input

`ScatterSeriesInput`

#### Returns

`this`

***

### annotateText()

> **annotateText**(`x`, `y`, `text`, `style?`): `this`

Defined in: [index.ts:1543](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1543)

Adds a text annotation at data coordinates — a reference-line label, a
peak marker. The default is 10pt black, so pass a `color` when the plot
uses a dark theme.

#### Parameters

##### x

`number`

##### y

`number`

##### text

`string`

##### style?

[`TextAnnotationStyle`](../../shared/interfaces/TextAnnotationStyle.md)

#### Returns

`this`

***

### bar()

> **bar**(`input`): `this`

Defined in: [index.ts:1652](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1652)

#### Parameters

##### input

`BarSeriesInput`

#### Returns

`this`

***

### boxplot()

> **boxplot**(`input`, `style?`): `this`

Defined in: [index.ts:1679](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1679)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md) \| [`ObservableSeries`](ObservableSeries.md)

##### style?

[`BoxplotSeriesStyle`](../../shared/type-aliases/BoxplotSeriesStyle.md)

#### Returns

`this`

***

### clone()

> **clone**(): `PlotBuilder`

Defined in: [index.ts:1828](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1828)

#### Returns

`PlotBuilder`

***

### contour()

> **contour**(`input`): `this`

Defined in: [index.ts:1756](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1756)

#### Parameters

##### input

`ContourInput`

#### Returns

`this`

***

### dispose()

> **dispose**(): `void`

Defined in: [index.ts:1832](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1832)

#### Returns

`void`

***

### dpi()

> **dpi**(`dpi`): `this`

Defined in: [index.ts:1339](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1339)

Sets the output DPI, scaling the pixels exported from the figure size.

#### Parameters

##### dpi

`number`

#### Returns

`this`

***

### ecdf()

> **ecdf**(`input`, `style?`): `this`

Defined in: [index.ts:1746](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1746)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### style?

[`StrokedSeriesStyle`](../../shared/type-aliases/StrokedSeriesStyle.md)

#### Returns

`this`

***

### errorBars()

> **errorBars**(`input`): `this`

Defined in: [index.ts:1693](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1693)

#### Parameters

##### input

`ErrorBarsInput`

#### Returns

`this`

***

### errorBarsXY()

> **errorBarsXY**(`input`): `this`

Defined in: [index.ts:1711](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1711)

#### Parameters

##### input

`ErrorBarsXYInput`

#### Returns

`this`

***

### figure()

> **figure**(`options`): `this`

Defined in: [index.ts:1201](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1201)

Applies figure-level presentation settings in one call.

These are chosen as a set — a journal's column width, body point size and
rule weight are one decision — so they are grouped rather than spread over
individual chained setters. Omitted fields keep their current value.

The call is atomic: every option is validated before any is applied, so a
rejected value leaves the plot unchanged.

```ts
plot.figure({ size: [3.25, 2.5], dpi: 300, fontSize: 9, fontFamily: "serif" });
```

#### Parameters

##### options

[`FigureOptions`](../../shared/interfaces/FigureOptions.md)

#### Returns

`this`

***

### grid()

> **grid**(`enabled?`): `this`

Defined in: [index.ts:1415](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1415)

#### Parameters

##### enabled?

`boolean` = `true`

#### Returns

`this`

***

### heatmap()

> **heatmap**(`input`): `this`

Defined in: [index.ts:1686](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1686)

#### Parameters

##### input

readonly [`NumericArray`](../../shared/type-aliases/NumericArray.md)[]

#### Returns

`this`

***

### histogram()

> **histogram**(`input`, `style?`): `this`

Defined in: [index.ts:1668](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1668)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md) \| [`ObservableSeries`](ObservableSeries.md)

##### style?

[`HistogramSeriesStyle`](../../shared/type-aliases/HistogramSeriesStyle.md)

#### Returns

`this`

***

### hline()

> **hline**(`y`, `style?`): `this`

Defined in: [index.ts:1528](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1528)

Adds a horizontal reference line spanning the plot width at data
y-coordinate `y`. Without a style it renders as the core default, a 1pt
dashed gray.

#### Parameters

##### y

`number`

##### style?

[`ReferenceLineStyle`](../../shared/interfaces/ReferenceLineStyle.md)

#### Returns

`this`

***

### invertX()

> **invertX**(): `this`

Defined in: [index.ts:1446](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1446)

Flip the x axis after its range resolves, so it runs high-to-low.

#### Returns

`this`

***

### invertY()

> **invertY**(): `this`

Defined in: [index.ts:1458](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1458)

Flip the y axis after its range resolves.

On a horizontal categorical chart this puts the first category at the
top, so ranked bars read in the order they were given.

#### Returns

`this`

***

### kde()

> **kde**(`input`, `style?`): `this`

Defined in: [index.ts:1736](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1736)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### style?

[`KdeSeriesStyle`](../../shared/type-aliases/KdeSeriesStyle.md)

#### Returns

`this`

***

### legend()

> **legend**(`position?`): `this`

Defined in: [index.ts:1404](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1404)

#### Parameters

##### position?

`"best"` \| `"upper_right"` \| `"upper_left"` \| `"lower_left"` \| `"lower_right"` \| `"right"` \| `"center_left"` \| `"center_right"` \| `"lower_center"` \| `"upper_center"` \| `"center"` \| `"outside_right"` \| `"outside_left"` \| `"outside_upper"` \| `"outside_lower"`

#### Returns

`this`

***

### line()

> **line**(`input`): `this`

Defined in: [index.ts:1636](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1636)

#### Parameters

##### input

`LineSeriesInput`

#### Returns

`this`

***

### mount()

> **mount**(`canvas`, `options?`): `Promise`\<[`CanvasSession`](CanvasSession.md)\>

Defined in: [index.ts:1901](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1901)

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

Defined in: [index.ts:1908](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1908)

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

Defined in: [index.ts:1768](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1768)

#### Parameters

##### values

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### labelsInput?

readonly `string`[] \| `ArrayLike`\<`string`\>

#### Returns

`this`

***

### polarLine()

> **polarLine**(`input`): `this`

Defined in: [index.ts:1812](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1812)

#### Parameters

##### input

`PolarLineInput`

#### Returns

`this`

***

### radar()

> **radar**(`input`): `this`

Defined in: [index.ts:1779](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1779)

#### Parameters

##### input

`RadarInput`

#### Returns

`this`

***

### renderPng()

> **renderPng**(): `Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

Defined in: [index.ts:1851](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1851)

#### Returns

`Promise`\<`Uint8Array`\<`ArrayBufferLike`\>\>

***

### renderSvg()

> **renderSvg**(): `Promise`\<`string`\>

Defined in: [index.ts:1869](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1869)

#### Returns

`Promise`\<`string`\>

***

### save()

> **save**(`options?`): `Promise`\<`void`\>

Defined in: [index.ts:1887](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1887)

#### Parameters

##### options?

[`PlotSaveOptions`](../../shared/interfaces/PlotSaveOptions.md)

#### Returns

`Promise`\<`void`\>

***

### scatter()

> **scatter**(`input`): `this`

Defined in: [index.ts:1644](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1644)

#### Parameters

##### input

`ScatterSeriesInput`

#### Returns

`this`

***

### setDpi()

> **setDpi**(`dpi`): `this`

Defined in: [index.ts:1343](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1343)

#### Parameters

##### dpi

`number`

#### Returns

`this`

***

### setGrid()

> **setGrid**(`enabled?`): `this`

Defined in: [index.ts:1419](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1419)

#### Parameters

##### enabled?

`boolean` = `true`

#### Returns

`this`

***

### setLegend()

> **setLegend**(`position?`): `this`

Defined in: [index.ts:1408](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1408)

#### Parameters

##### position?

`"best"` \| `"upper_right"` \| `"upper_left"` \| `"lower_left"` \| `"lower_right"` \| `"right"` \| `"center_left"` \| `"center_right"` \| `"lower_center"` \| `"upper_center"` \| `"center"` \| `"outside_right"` \| `"outside_left"` \| `"outside_upper"` \| `"outside_lower"`

#### Returns

`this`

***

### setSizePx()

> **setSizePx**(`width`, `height`): `this`

Defined in: [index.ts:1181](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1181)

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

Defined in: [index.ts:1357](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1357)

#### Parameters

##### theme

`"light"` \| `"dark"` \| `"seaborn"` \| `"publication"` \| `"minimal"` \| `"presentation"`

#### Returns

`this`

***

### setTicks()

> **setTicks**(`enabled`): `this`

Defined in: [index.ts:1368](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1368)

#### Parameters

##### enabled

`boolean`

#### Returns

`this`

***

### setTitle()

> **setTitle**(`title`): `this`

Defined in: [index.ts:1378](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1378)

#### Parameters

##### title

`string`

#### Returns

`this`

***

### setXLabel()

> **setXLabel**(`label`): `this`

Defined in: [index.ts:1388](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1388)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### setXLim()

> **setXLim**(`minimum`, `maximum`): `this`

Defined in: [index.ts:1429](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1429)

#### Parameters

##### minimum

`number`

##### maximum

`number`

#### Returns

`this`

***

### setXScale()

> **setXScale**(`scale`, `linthresh?`): `this`

Defined in: [index.ts:1468](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1468)

#### Parameters

##### scale

`"linear"` \| `"log"` \| `"symlog"`

##### linthresh?

`number`

#### Returns

`this`

***

### setYLabel()

> **setYLabel**(`label`): `this`

Defined in: [index.ts:1398](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1398)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### setYLim()

> **setYLim**(`minimum`, `maximum`): `this`

Defined in: [index.ts:1439](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1439)

#### Parameters

##### minimum

`number`

##### maximum

`number`

#### Returns

`this`

***

### setYScale()

> **setYScale**(`scale`, `linthresh?`): `this`

Defined in: [index.ts:1478](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1478)

#### Parameters

##### scale

`"linear"` \| `"log"` \| `"symlog"`

##### linthresh?

`number`

#### Returns

`this`

***

### sizePx()

> **sizePx**(`width`, `height`): `this`

Defined in: [index.ts:1177](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1177)

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

Defined in: [index.ts:1353](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1353)

#### Parameters

##### theme

`"light"` \| `"dark"` \| `"seaborn"` \| `"publication"` \| `"minimal"` \| `"presentation"`

#### Returns

`this`

***

### ticks()

> **ticks**(`enabled`): `this`

Defined in: [index.ts:1364](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1364)

#### Parameters

##### enabled

`boolean`

#### Returns

`this`

***

### title()

> **title**(`title`): `this`

Defined in: [index.ts:1374](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1374)

#### Parameters

##### title

`string`

#### Returns

`this`

***

### toSnapshot()

> **toSnapshot**(): [`PlotSnapshot`](../../shared/interfaces/PlotSnapshot.md)

Defined in: [index.ts:1841](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1841)

#### Returns

[`PlotSnapshot`](../../shared/interfaces/PlotSnapshot.md)

***

### violin()

> **violin**(`input`, `style?`): `this`

Defined in: [index.ts:1802](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1802)

#### Parameters

##### input

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

##### style?

[`StrokedSeriesStyle`](../../shared/type-aliases/StrokedSeriesStyle.md)

#### Returns

`this`

***

### vline()

> **vline**(`x`, `style?`): `this`

Defined in: [index.ts:1513](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1513)

Adds a vertical reference line spanning the plot height at data
x-coordinate `x` — an absorption edge, a threshold, a boundary. Without a
style it renders as the core default, a 1pt dashed gray.

#### Parameters

##### x

`number`

##### style?

[`ReferenceLineStyle`](../../shared/interfaces/ReferenceLineStyle.md)

#### Returns

`this`

***

### xlabel()

> **xlabel**(`label`): `this`

Defined in: [index.ts:1384](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1384)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### xlim()

> **xlim**(`minimum`, `maximum`): `this`

Defined in: [index.ts:1425](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1425)

#### Parameters

##### minimum

`number`

##### maximum

`number`

#### Returns

`this`

***

### xscale()

> **xscale**(`scale`, `linthresh?`): `this`

Defined in: [index.ts:1464](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1464)

#### Parameters

##### scale

`"linear"` \| `"log"` \| `"symlog"`

##### linthresh?

`number`

#### Returns

`this`

***

### ylabel()

> **ylabel**(`label`): `this`

Defined in: [index.ts:1394](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1394)

#### Parameters

##### label

`string`

#### Returns

`this`

***

### ylim()

> **ylim**(`minimum`, `maximum`): `this`

Defined in: [index.ts:1435](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1435)

#### Parameters

##### minimum

`number`

##### maximum

`number`

#### Returns

`this`

***

### yscale()

> **yscale**(`scale`, `linthresh?`): `this`

Defined in: [index.ts:1474](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1474)

#### Parameters

##### scale

`"linear"` \| `"log"` \| `"symlog"`

##### linthresh?

`number`

#### Returns

`this`

***

### fromSnapshot()

> `static` **fromSnapshot**(`snapshot`): `PlotBuilder`

Defined in: [index.ts:1037](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L1037)

#### Parameters

##### snapshot

[`PlotSnapshot`](../../shared/interfaces/PlotSnapshot.md)

#### Returns

`PlotBuilder`
