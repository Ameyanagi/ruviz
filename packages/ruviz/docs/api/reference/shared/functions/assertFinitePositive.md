[**ruviz**](../../README.md)

***

[ruviz](../../README.md) / [shared](../README.md) / assertFinitePositive

# Function: assertFinitePositive()

> **assertFinitePositive**(`value`, `label`): `void`

Defined in: [shared.ts:580](https://github.com/Ameyanagi/ruviz/blob/main/packages/ruviz/src/shared.ts#L580)

Reject values the core builders would silently clamp to a default. Clamping
is right for Rust call sites, but from JS an `undefined` arrives as NaN and
would otherwise produce a wrong figure with no diagnostic.

## Parameters

### value

`number`

### label

`string`

## Returns

`void`
