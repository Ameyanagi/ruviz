[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / SineSignal

# Class: SineSignal

Defined in: [index.ts:713](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L713)

Procedural sine-wave signal for temporal playback in interactive sessions.

## Constructors

### Constructor

> **new SineSignal**(`options`): `SineSignal`

Defined in: [index.ts:717](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L717)

#### Parameters

##### options

[`SineSignalOptions`](../../shared/interfaces/SineSignalOptions.md)

#### Returns

`SineSignal`

## Properties

### options

> `readonly` **options**: [`NormalizedSineSignalOptions`](../../shared/interfaces/NormalizedSineSignalOptions.md)

Defined in: [index.ts:714](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L714)

## Accessors

### length

#### Get Signature

> **get** **length**(): `number`

Defined in: [index.ts:722](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L722)

##### Returns

`number`

## Methods

### \_toRawSignal()

> **\_toRawSignal**(`module?`): `Promise`\<`SignalVecF64`\>

Defined in: [index.ts:750](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L750)

#### Parameters

##### module?

`__module`

#### Returns

`Promise`\<`SignalVecF64`\>

***

### valuesAt()

> **valuesAt**(`timeSeconds`): `Float64Array`

Defined in: [index.ts:726](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L726)

#### Parameters

##### timeSeconds

`number`

#### Returns

`Float64Array`
