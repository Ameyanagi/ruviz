[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / ObservableSeries

# Class: ObservableSeries

Defined in: [index.ts:680](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L680)

Mutable numeric data source for reactive plot updates.

## Constructors

### Constructor

> **new ObservableSeries**(`values`): `ObservableSeries`

Defined in: [index.ts:684](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L684)

#### Parameters

##### values

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`ObservableSeries`

## Accessors

### length

#### Get Signature

> **get** **length**(): `number`

Defined in: [index.ts:689](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L689)

##### Returns

`number`

## Methods

### \_toRawObservable()

> **\_toRawObservable**(`module?`): `Promise`\<`ObservableVecF64`\>

Defined in: [index.ts:719](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L719)

#### Parameters

##### module?

`__module`

#### Returns

`Promise`\<`ObservableVecF64`\>

***

### replace()

> **replace**(`values`): `void`

Defined in: [index.ts:693](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L693)

#### Parameters

##### values

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`void`

***

### setAt()

> **setAt**(`index`, `value`): `void`

Defined in: [index.ts:702](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L702)

#### Parameters

##### index

`number`

##### value

`number`

#### Returns

`void`

***

### snapshotValues()

> **snapshotValues**(): `number`[]

Defined in: [index.ts:715](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L715)

#### Returns

`number`[]

***

### values()

> **values**(): `Float64Array`

Defined in: [index.ts:711](https://github.com/Ameyanagi/ruviz/blob/91f8e7b36952093ba0bf232fc1ad623212329596/packages/ruviz-web/src/index.ts#L711)

#### Returns

`Float64Array`
