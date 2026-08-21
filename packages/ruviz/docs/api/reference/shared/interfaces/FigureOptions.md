[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [shared](../README.md) / FigureOptions

# Interface: FigureOptions

Defined in: [shared.ts:550](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L550)

Figure-level presentation settings, applied together by `PlotBuilder.figure`.

These are grouped rather than exposed as individual chained setters because
they are chosen as a set — a journal's column width, body point size and rule
weight are one decision, not eight. Every field is optional; omitting one
leaves the current value untouched.

## Properties

### dpi?

> `optional` **dpi?**: `number`

Defined in: [shared.ts:554](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L554)

Output DPI, applied after `size` to fix the exported pixel dimensions.

***

### fontFamily?

> `optional` **fontFamily?**: `string`

Defined in: [shared.ts:560](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L560)

`serif` | `sans-serif` | `monospace` | `cursive` | `fantasy`, or a registered family.

***

### fontSize?

> `optional` **fontSize?**: `number`

Defined in: [shared.ts:556](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L556)

Base text size in points; other text sizes derive from it.

***

### lineWidthPt?

> `optional` **lineWidthPt?**: `number`

Defined in: [shared.ts:564](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L564)

Data line width in points.

***

### margin?

> `optional` **margin?**: `number`

Defined in: [shared.ts:566](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L566)

Plot margin as a fraction of the figure, 0.0–0.5.

***

### maxResolution?

> `optional` **maxResolution?**: \[`number`, `number`\]

Defined in: [shared.ts:572](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L572)

Cap exported pixel dimensions while preserving aspect ratio.

***

### scaleTypography?

> `optional` **scaleTypography?**: `number`

Defined in: [shared.ts:562](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L562)

Scales every text size at once, preserving the typographic hierarchy.

***

### scientificNotation?

> `optional` **scientificNotation?**: `boolean`

Defined in: [shared.ts:570](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L570)

Scientific notation on axis tick labels.

***

### size?

> `optional` **size?**: \[`number`, `number`\]

Defined in: [shared.ts:552](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L552)

Figure size in inches, `[width, height]` — the unit journals specify.

***

### tightLayoutPad?

> `optional` **tightLayoutPad?**: `number`

Defined in: [shared.ts:568](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L568)

Shrink margins to fit the text, leaving this many points of slack.

***

### titleSize?

> `optional` **titleSize?**: `number`

Defined in: [shared.ts:558](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L558)

Title size in points, absolute rather than relative to `fontSize`.
