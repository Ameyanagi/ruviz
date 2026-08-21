[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [shared](../README.md) / SeriesStyleSnapshot

# Interface: SeriesStyleSnapshot

Defined in: [shared.ts:83](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L83)

Per-series styling. Keys are camelCase to match the snapshot spelling shared
with the Python binding; each maps to one core plot builder setter.

## Properties

### alpha?

> `optional` **alpha?**: `number`

Defined in: [shared.ts:86](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L86)

***

### bandwidth?

> `optional` **bandwidth?**: `number`

Defined in: [shared.ts:99](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L99)

***

### bins?

> `optional` **bins?**: `number`

Defined in: [shared.ts:91](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L91)

***

### color?

> `optional` **color?**: `string`

Defined in: [shared.ts:85](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L85)

***

### density?

> `optional` **density?**: `boolean`

Defined in: [shared.ts:98](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L98)

On a histogram: normalize bars to a probability density, so a KDE overlay
lines up. On a scatter: render through plot-area density aggregation
instead of compositing every marker — an opt-in approximation for very
large series whose cost scales with pixels, not points.

***

### label?

> `optional` **label?**: `string`

Defined in: [shared.ts:84](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L84)

***

### levels?

> `optional` **levels?**: `number`

Defined in: [shared.ts:100](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L100)

***

### linestyle?

> `optional` **linestyle?**: `"solid"` \| `"dashed"` \| `"dotted"` \| `"dash-dot"` \| `"dash-dot-dot"`

Defined in: [shared.ts:88](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L88)

***

### marker?

> `optional` **marker?**: `"circle"` \| `"square"` \| `"triangle"` \| `"triangle-down"` \| `"diamond"` \| `"plus"` \| `"cross"` \| `"star"` \| `"circle-open"` \| `"square-open"` \| `"triangle-open"` \| `"diamond-open"`

Defined in: [shared.ts:89](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L89)

***

### markerSize?

> `optional` **markerSize?**: `number`

Defined in: [shared.ts:90](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L90)

***

### orientation?

> `optional` **orientation?**: `"vertical"` \| `"horizontal"`

Defined in: [shared.ts:101](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L101)

***

### showMean?

> `optional` **showMean?**: `boolean`

Defined in: [shared.ts:103](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L103)

Box plots only: draw a diamond at the sample mean.

***

### width?

> `optional` **width?**: `number`

Defined in: [shared.ts:87](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L87)

***

### widthRatio?

> `optional` **widthRatio?**: `number`

Defined in: [shared.ts:105](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L105)

Box plots only: box width as a fraction of its category slot (0..=1).
