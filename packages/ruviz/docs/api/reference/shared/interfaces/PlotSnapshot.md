[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [shared](../README.md) / PlotSnapshot

# Interface: PlotSnapshot

Defined in: [shared.ts:369](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L369)

## Properties

### annotations?

> `optional` **annotations?**: [`PlotAnnotationSnapshot`](../type-aliases/PlotAnnotationSnapshot.md)[]

Defined in: [shared.ts:414](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L414)

Plot-level annotations — reference lines and text labels — in call order.

***

### dpi?

> `optional` **dpi?**: `number`

Defined in: [shared.ts:380](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L380)

Output DPI; applied after the figure size, so raising it scales the exported pixels.

***

### fontFamily?

> `optional` **fontFamily?**: `string`

Defined in: [shared.ts:386](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L386)

`serif` | `sans-serif` | `monospace` | `cursive` | `fantasy`, or a registered family name.

***

### fontSize?

> `optional` **fontSize?**: `number`

Defined in: [shared.ts:382](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L382)

Base text size in points; every other text size derives from it.

***

### grid?

> `optional` **grid?**: `boolean`

Defined in: [shared.ts:405](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L405)

***

### invertX?

> `optional` **invertX?**: `boolean`

Defined in: [shared.ts:409](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L409)

Present (and true) only when the axis was flipped high-to-low.

***

### invertY?

> `optional` **invertY?**: `boolean`

Defined in: [shared.ts:410](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L410)

***

### legend?

> `optional` **legend?**: `"best"` \| `"upper_right"` \| `"upper_left"` \| `"lower_left"` \| `"lower_right"` \| `"right"` \| `"center_left"` \| `"center_right"` \| `"lower_center"` \| `"upper_center"` \| `"center"` \| `"outside_right"` \| `"outside_left"` \| `"outside_upper"` \| `"outside_lower"`

Defined in: [shared.ts:404](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L404)

***

### lineWidthPt?

> `optional` **lineWidthPt?**: `number`

Defined in: [shared.ts:390](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L390)

Data line width in points.

***

### margin?

> `optional` **margin?**: `number`

Defined in: [shared.ts:392](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L392)

Plot margin as a fraction of the figure, 0.0–0.5.

***

### maxResolution?

> `optional` **maxResolution?**: \[`number`, `number`\]

Defined in: [shared.ts:398](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L398)

Caps exported pixel dimensions while preserving aspect ratio.

***

### scaleTypography?

> `optional` **scaleTypography?**: `number`

Defined in: [shared.ts:388](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L388)

Scales every text size at once, preserving the typographic hierarchy.

***

### schemaVersion?

> `optional` **schemaVersion?**: `number`

Defined in: [shared.ts:371](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L371)

Snapshot layout version; absent on snapshots written before it existed.

***

### scientificNotation?

> `optional` **scientificNotation?**: `boolean`

Defined in: [shared.ts:396](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L396)

Scientific notation on axis tick labels.

***

### series

> **series**: [`PlotSeriesSnapshot`](../type-aliases/PlotSeriesSnapshot.md)[]

Defined in: [shared.ts:415](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L415)

***

### sizeIn?

> `optional` **sizeIn?**: \[`number`, `number`\]

Defined in: [shared.ts:378](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L378)

Figure size in inches — the unit journals specify (single column is
typically 3.25, double column 6.5). Applied before `dpi`, which then fixes
the exported pixel dimensions. Takes precedence over `sizePx`.

***

### sizePx?

> `optional` **sizePx?**: \[`number`, `number`\]

Defined in: [shared.ts:372](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L372)

***

### theme?

> `optional` **theme?**: `"light"` \| `"dark"` \| `"seaborn"` \| `"publication"` \| `"minimal"` \| `"presentation"`

Defined in: [shared.ts:399](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L399)

***

### ticks?

> `optional` **ticks?**: `boolean`

Defined in: [shared.ts:400](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L400)

***

### tightLayoutPad?

> `optional` **tightLayoutPad?**: `number`

Defined in: [shared.ts:394](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L394)

Shrink margins to fit text, leaving this many points of slack.

***

### title?

> `optional` **title?**: `string`

Defined in: [shared.ts:401](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L401)

***

### titleSize?

> `optional` **titleSize?**: `number`

Defined in: [shared.ts:384](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L384)

Title size in points, absolute rather than relative to `fontSize`.

***

### xLabel?

> `optional` **xLabel?**: `string`

Defined in: [shared.ts:402](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L402)

***

### xLim?

> `optional` **xLim?**: \[`number`, `number`\]

Defined in: [shared.ts:406](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L406)

***

### xScale?

> `optional` **xScale?**: [`AxisScaleSnapshot`](../type-aliases/AxisScaleSnapshot.md)

Defined in: [shared.ts:411](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L411)

***

### yLabel?

> `optional` **yLabel?**: `string`

Defined in: [shared.ts:403](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L403)

***

### yLim?

> `optional` **yLim?**: \[`number`, `number`\]

Defined in: [shared.ts:407](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L407)

***

### yScale?

> `optional` **yScale?**: [`AxisScaleSnapshot`](../type-aliases/AxisScaleSnapshot.md)

Defined in: [shared.ts:412](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L412)
