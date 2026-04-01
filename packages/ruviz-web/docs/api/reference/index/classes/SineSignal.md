[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / SineSignal

# Class: SineSignal

Defined in: [index.ts:731](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L731)

Procedural sine-wave signal for temporal playback in interactive sessions.

## Constructors

### Constructor

> **new SineSignal**(`options`): `SineSignal`

Defined in: [index.ts:735](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L735)

#### Parameters

##### options

[`SineSignalOptions`](../../shared/interfaces/SineSignalOptions.md)

#### Returns

`SineSignal`

## Properties

### options

> `readonly` **options**: [`NormalizedSineSignalOptions`](../../shared/interfaces/NormalizedSineSignalOptions.md)

Defined in: [index.ts:732](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L732)

## Accessors

### length

#### Get Signature

> **get** **length**(): `number`

Defined in: [index.ts:740](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L740)

##### Returns

`number`

## Methods

### \_toRawSignal()

> **\_toRawSignal**(`module?`): `Promise`\<`SignalVecF64`\>

Defined in: [index.ts:768](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L768)

#### Parameters

##### module?

`__module`

#### Returns

`Promise`\<`SignalVecF64`\>

***

### valuesAt()

> **valuesAt**(`timeSeconds`): `Float64Array`

Defined in: [index.ts:744](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L744)

#### Parameters

##### timeSeconds

`number`

#### Returns

`Float64Array`
