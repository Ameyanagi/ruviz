[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / ObservableSeries

# Class: ObservableSeries

Defined in: [index.ts:662](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L662)

Mutable numeric data source for reactive plot updates.

## Constructors

### Constructor

> **new ObservableSeries**(`values`): `ObservableSeries`

Defined in: [index.ts:666](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L666)

#### Parameters

##### values

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`ObservableSeries`

## Accessors

### length

#### Get Signature

> **get** **length**(): `number`

Defined in: [index.ts:671](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L671)

##### Returns

`number`

## Methods

### \_toRawObservable()

> **\_toRawObservable**(`module?`): `Promise`\<`ObservableVecF64`\>

Defined in: [index.ts:701](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L701)

#### Parameters

##### module?

`__module`

#### Returns

`Promise`\<`ObservableVecF64`\>

***

### replace()

> **replace**(`values`): `void`

Defined in: [index.ts:675](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L675)

#### Parameters

##### values

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`void`

***

### setAt()

> **setAt**(`index`, `value`): `void`

Defined in: [index.ts:684](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L684)

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

Defined in: [index.ts:697](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L697)

#### Returns

`number`[]

***

### values()

> **values**(): `Float64Array`

Defined in: [index.ts:693](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz-web/src/index.ts#L693)

#### Returns

`Float64Array`
