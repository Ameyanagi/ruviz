[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [index](../README.md) / ObservableSeries

# Class: ObservableSeries

Defined in: [index.ts:850](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L850)

Mutable numeric data source for reactive plot updates.

## Constructors

### Constructor

> **new ObservableSeries**(`values`): `ObservableSeries`

Defined in: [index.ts:854](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L854)

#### Parameters

##### values

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`ObservableSeries`

## Accessors

### length

#### Get Signature

> **get** **length**(): `number`

Defined in: [index.ts:859](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L859)

##### Returns

`number`

## Methods

### \_toRawObservable()

> **\_toRawObservable**(`module?`): `Promise`\<`ObservableVecF64`\>

Defined in: [index.ts:889](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L889)

#### Parameters

##### module?

`__module`

#### Returns

`Promise`\<`ObservableVecF64`\>

***

### replace()

> **replace**(`values`): `void`

Defined in: [index.ts:863](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L863)

#### Parameters

##### values

[`NumericArray`](../../shared/type-aliases/NumericArray.md)

#### Returns

`void`

***

### setAt()

> **setAt**(`index`, `value`): `void`

Defined in: [index.ts:872](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L872)

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

Defined in: [index.ts:885](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L885)

#### Returns

`number`[]

***

### values()

> **values**(): `Float64Array`

Defined in: [index.ts:881](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/index.ts#L881)

#### Returns

`Float64Array`
