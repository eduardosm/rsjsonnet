/*
Copyright 2015 Google Inc. All rights reserved.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

// This is the part of the standard library that is written in Jsonnet.
// It is based on the standard library from the C++ implementation of Jsonnet
// (version 0.20.0), with the above license.
// The functions that are implemented as built-in have been removed.

{
  local std = self,

  repeat(what, count)::
    local joiner =
      if std.isString(what) then ''
      else if std.isArray(what) then []
      else error 'std.repeat first argument must be an array or a string';
    std.join(joiner, std.makeArray(count, function(i) what)),

  mapWithKey(func, obj)::
    if !std.isFunction(func) then
      error ('std.mapWithKey first param must be function, got ' + std.type(func))
    else if !std.isObject(obj) then
      error ('std.mapWithKey second param must be object, got ' + std.type(obj))
    else
      { [k]: func(k, obj[k]) for k in std.objectFields(obj) },

  flatMap(func, arr)::
    if !std.isFunction(func) then
      error ('std.flatMap first param must be function, got ' + std.type(func))
    else if std.isArray(arr) then
      std.flattenArrays(std.makeArray(std.length(arr), function(i) func(arr[i])))
    else if std.isString(arr) then
      std.join('', std.makeArray(std.length(arr), function(i) func(arr[i])))
    else error ('std.flatMap second param must be array / string, got ' + std.type(arr)),

  lines(arr)::
    std.join('\n', arr + ['']),

  deepJoin(arr)::
    if std.isString(arr) then
      arr
    else if std.isArray(arr) then
      std.join('', [std.deepJoin(x) for x in arr])
    else
      error 'Expected string or array, got %s' % std.type(arr),

  filterMap(filter_func, map_func, arr)::
    if !std.isFunction(filter_func) then
      error ('std.filterMap first param must be function, got ' + std.type(filter_func))
    else if !std.isFunction(map_func) then
      error ('std.filterMap second param must be function, got ' + std.type(map_func))
    else if !std.isArray(arr) then
      error ('std.filterMap third param must be array, got ' + std.type(arr))
    else
      std.map(map_func, std.filter(filter_func, arr)),

  abs(n)::
    if !std.isNumber(n) then
      error 'std.abs expected number, got ' + std.type(n)
    else
      if n > 0 then n else -n,

  sign(n)::
    if !std.isNumber(n) then
      error 'std.sign expected number, got ' + std.type(n)
    else
      if n > 0 then
        1
      else if n < 0 then
        -1
      else 0,

  max(a, b)::
    if !std.isNumber(a) then
      error 'std.max first param expected number, got ' + std.type(a)
    else if !std.isNumber(b) then
      error 'std.max second param expected number, got ' + std.type(b)
    else
      if a > b then a else b,

  min(a, b)::
    if !std.isNumber(a) then
      error 'std.min first param expected number, got ' + std.type(a)
    else if !std.isNumber(b) then
      error 'std.min second param expected number, got ' + std.type(b)
    else
      if a < b then a else b,

  clamp(x, minVal, maxVal)::
    if x < minVal then minVal
    else if x > maxVal then maxVal
    else x,

  flattenArrays(arrs)::
    std.foldl(function(a, b) a + b, arrs, []),

  manifestToml(value):: std.manifestTomlEx(value, '  '),

  manifestJson(value):: std.manifestJsonEx(value, '    '),

  manifestJsonMinified(value):: std.manifestJsonEx(value, '', '', ':'),

  get(o, f, default=null, inc_hidden=true)::
    if std.objectHasEx(o, f, inc_hidden) then o[f] else default,

  objectFields(o)::
    std.objectFieldsEx(o, false),

  objectFieldsAll(o)::
    std.objectFieldsEx(o, true),

  objectHas(o, f)::
    std.objectHasEx(o, f, false),

  objectHasAll(o, f)::
    std.objectHasEx(o, f, true),

  objectValues(o)::
    [o[k] for k in std.objectFields(o)],

  objectValuesAll(o)::
    [o[k] for k in std.objectFieldsAll(o)],

  objectKeysValues(o)::
    [{ key: k, value: o[k] } for k in std.objectFields(o)],

  objectKeysValuesAll(o)::
    [{ key: k, value: o[k] } for k in std.objectFieldsAll(o)],

  resolvePath(f, r)::
    local arr = std.split(f, '/');
    std.join('/', std.makeArray(std.length(arr) - 1, function(i) arr[i]) + [r]),

  __array_less(arr1, arr2):: std.__compare_array(arr1, arr2) == -1,
  __array_greater(arr1, arr2):: std.__compare_array(arr1, arr2) == 1,
  __array_less_or_equal(arr1, arr2):: std.__compare_array(arr1, arr2) <= 0,
  __array_greater_or_equal(arr1, arr2):: std.__compare_array(arr1, arr2) >= 0,

  sum(arr):: std.foldl(function(a, b) a + b, arr, 0),

  xor(x, y):: x != y,

  xnor(x, y):: x == y,

  round(x):: std.floor(x + 0.5),

  isEmpty(str):: std.length(str) == 0,
}
