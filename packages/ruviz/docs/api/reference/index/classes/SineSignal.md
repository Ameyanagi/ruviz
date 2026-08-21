[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / SineSignal

# Class: SineSignal

Defined in: [index.ts:901](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L901)

Procedural sine-wave signal for temporal playback in interactive sessions.

## Constructors

### Constructor

> **new SineSignal**(`options`): `SineSignal`

Defined in: [index.ts:905](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L905)

#### Parameters

##### options

[`SineSignalOptions`](../../shared/interfaces/SineSignalOptions.md)

#### Returns

`SineSignal`

## Properties

### options

> `readonly` **options**: [`NormalizedSineSignalOptions`](../../shared/interfaces/NormalizedSineSignalOptions.md)

Defined in: [index.ts:902](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L902)

## Accessors

### length

#### Get Signature

> **get** **length**(): `number`

Defined in: [index.ts:910](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L910)

##### Returns

`number`

## Methods

### \_toRawSignal()

> **\_toRawSignal**(`module?`): `Promise`\<`SignalVecF64`\>

Defined in: [index.ts:938](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L938)

#### Parameters

##### module?

`__module`

#### Returns

`Promise`\<`SignalVecF64`\>

***

### valuesAt()

> **valuesAt**(`timeSeconds`): `Float64Array`

Defined in: [index.ts:914](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L914)

#### Parameters

##### timeSeconds

`number`

#### Returns

`Float64Array`
