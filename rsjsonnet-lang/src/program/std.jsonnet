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

  lstripChars(str, chars)::
    if std.length(str) > 0 && std.member(chars, str[0]) then
      std.lstripChars(str[1:], chars) tailstrict
    else
      str,

  rstripChars(str, chars)::
    local len = std.length(str);
    if len > 0 && std.member(chars, str[len - 1]) then
      std.rstripChars(str[:len - 1], chars) tailstrict
    else
      str,

  stripChars(str, chars)::
    std.lstripChars(std.rstripChars(str, chars), chars),

  repeat(what, count)::
    local joiner =
      if std.isString(what) then ''
      else if std.isArray(what) then []
      else error 'std.repeat first argument must be an array or a string';
    std.join(joiner, std.makeArray(count, function(i) what)),

  member(arr, x)::
    if std.isArray(arr) then
      std.count(arr, x) > 0
    else if std.isString(arr) then
      std.length(std.findSubstr(x, arr)) > 0
    else error 'std.member first argument must be an array or a string',

  count(arr, x):: std.length(std.filter(function(v) v == x, arr)),

  mod(a, b)::
    if std.isNumber(a) && std.isNumber(b) then
      std.modulo(a, b)
    else if std.isString(a) then
      std.format(a, b)
    else
      error 'Operator % cannot be used on types ' + std.type(a) + ' and ' + std.type(b) + '.',

  map(func, arr)::
    if !std.isFunction(func) then
      error ('std.map first param must be function, got ' + std.type(func))
    else if !std.isArray(arr) && !std.isString(arr) then
      error ('std.map second param must be array / string, got ' + std.type(arr))
    else
      std.makeArray(std.length(arr), function(i) func(arr[i])),

  mapWithIndex(func, arr)::
    if !std.isFunction(func) then
      error ('std.mapWithIndex first param must be function, got ' + std.type(func))
    else if !std.isArray(arr) && !std.isString(arr) then
      error ('std.mapWithIndex second param must be array, got ' + std.type(arr))
    else
      std.makeArray(std.length(arr), function(i) func(i, arr[i])),

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

  manifestIni(ini)::
    local body_lines(body) =
      std.join([], [
        local value_or_values = body[k];
        if std.isArray(value_or_values) then
          ['%s = %s' % [k, value] for value in value_or_values]
        else
          ['%s = %s' % [k, value_or_values]]

        for k in std.objectFields(body)
      ]);

    local section_lines(sname, sbody) = ['[%s]' % [sname]] + body_lines(sbody),
          main_body = if std.objectHas(ini, 'main') then body_lines(ini.main) else [],
          all_sections = [
      section_lines(k, ini.sections[k])
      for k in std.objectFields(ini.sections)
    ];
    std.join('\n', main_body + std.flattenArrays(all_sections) + ['']),

  manifestToml(value):: std.manifestTomlEx(value, '  '),

  manifestTomlEx(value, indent)::
    local
      escapeStringToml = std.escapeStringJson,
      escapeKeyToml(key) =
        local bare_allowed = std.set(std.stringChars('ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-'));
        if std.setUnion(std.set(std.stringChars(key)), bare_allowed) == bare_allowed then key else escapeStringToml(key),
      isTableArray(v) = std.isArray(v) && std.length(v) > 0 && std.all(std.map(std.isObject, v)),
      isSection(v) = std.isObject(v) || isTableArray(v),
      renderValue(v, indexedPath, inline, cindent) =
        if v == true then
          'true'
        else if v == false then
          'false'
        else if v == null then
          error 'Tried to manifest "null" at ' + indexedPath
        else if std.isNumber(v) then
          '' + v
        else if std.isString(v) then
          escapeStringToml(v)
        else if std.isFunction(v) then
          error 'Tried to manifest function at ' + indexedPath
        else if std.isArray(v) then
          if std.length(v) == 0 then
            '[]'
          else
            local range = std.range(0, std.length(v) - 1);
            local new_indent = if inline then '' else cindent + indent;
            local separator = if inline then ' ' else '\n';
            local lines = ['[' + separator]
                          + std.join([',' + separator],
                                     [
                                       [new_indent + renderValue(v[i], indexedPath + [i], true, '')]
                                       for i in range
                                     ])
                          + [separator + (if inline then '' else cindent) + ']'];
            std.join('', lines)
        else if std.isObject(v) then
          local lines = ['{ ']
                        + std.join([', '],
                                   [
                                     [escapeKeyToml(k) + ' = ' + renderValue(v[k], indexedPath + [k], true, '')]
                                     for k in std.objectFields(v)
                                   ])
                        + [' }'];
          std.join('', lines),
      renderTableInternal(v, path, indexedPath, cindent) =
        local kvp = std.flattenArrays([
          [cindent + escapeKeyToml(k) + ' = ' + renderValue(v[k], indexedPath + [k], false, cindent)]
          for k in std.objectFields(v)
          if !isSection(v[k])
        ]);
        local sections = [std.join('\n', kvp)] + [
          (
            if std.isObject(v[k]) then
              renderTable(v[k], path + [k], indexedPath + [k], cindent)
            else
              renderTableArray(v[k], path + [k], indexedPath + [k], cindent)
          )
          for k in std.objectFields(v)
          if isSection(v[k])
        ];
        std.join('\n\n', sections),
      renderTable(v, path, indexedPath, cindent) =
        cindent + '[' + std.join('.', std.map(escapeKeyToml, path)) + ']'
        + (if v == {} then '' else '\n')
        + renderTableInternal(v, path, indexedPath, cindent + indent),
      renderTableArray(v, path, indexedPath, cindent) =
        local range = std.range(0, std.length(v) - 1);
        local sections = [
          (cindent + '[[' + std.join('.', std.map(escapeKeyToml, path)) + ']]'
           + (if v[i] == {} then '' else '\n')
           + renderTableInternal(v[i], path, indexedPath + [i], cindent + indent))
          for i in range
        ];
        std.join('\n\n', sections);
    if std.isObject(value) then
      renderTableInternal(value, [], [], '')
    else
      error 'TOML body must be an object. Got ' + std.type(value),

  escapeStringPython(str)::
    std.escapeStringJson(str),

  manifestJson(value):: std.manifestJsonEx(value, '    '),

  manifestJsonMinified(value):: std.manifestJsonEx(value, '', '', ':'),

  manifestPython(v)::
    if std.isObject(v) then
      local fields = [
        '%s: %s' % [std.escapeStringPython(k), std.manifestPython(v[k])]
        for k in std.objectFields(v)
      ];
      '{%s}' % [std.join(', ', fields)]
    else if std.isArray(v) then
      '[%s]' % [std.join(', ', [std.manifestPython(v2) for v2 in v])]
    else if std.isString(v) then
      '%s' % [std.escapeStringPython(v)]
    else if std.isFunction(v) then
      error 'cannot manifest function'
    else if std.isNumber(v) then
      std.toString(v)
    else if v == true then
      'True'
    else if v == false then
      'False'
    else if v == null then
      'None',

  manifestPythonVars(conf)::
    local vars = ['%s = %s' % [k, std.manifestPython(conf[k])] for k in std.objectFields(conf)];
    std.join('\n', vars + ['']),

  manifestXmlJsonml(value)::
    if !std.isArray(value) then
      error 'Expected a JSONML value (an array), got %s' % std.type(value)
    else
      local aux(v) =
        if std.isString(v) then
          v
        else
          local tag = v[0];
          local has_attrs = std.length(v) > 1 && std.isObject(v[1]);
          local attrs = if has_attrs then v[1] else {};
          local children = if has_attrs then v[2:] else v[1:];
          local attrs_str =
            std.join('', [' %s="%s"' % [k, attrs[k]] for k in std.objectFields(attrs)]);
          std.deepJoin(['<', tag, attrs_str, '>', [aux(x) for x in children], '</', tag, '>']);

      aux(value),

  mergePatch(target, patch)::
    if std.isObject(patch) then
      local target_object =
        if std.isObject(target) then target else {};

      local target_fields =
        if std.isObject(target_object) then std.objectFields(target_object) else [];

      local null_fields = [k for k in std.objectFields(patch) if patch[k] == null];
      local both_fields = std.setUnion(target_fields, std.objectFields(patch));

      {
        [k]:
          if !std.objectHas(patch, k) then
            target_object[k]
          else if !std.objectHas(target_object, k) then
            std.mergePatch(null, patch[k]) tailstrict
          else
            std.mergePatch(target_object[k], patch[k]) tailstrict
        for k in std.setDiff(both_fields, null_fields)
      }
    else
      patch,

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

  prune(a)::
    local isContent(b) =
      if b == null then
        false
      else if std.isArray(b) then
        std.length(b) > 0
      else if std.isObject(b) then
        std.length(b) > 0
      else
        true;
    if std.isArray(a) then
      [std.prune(x) for x in a if isContent($.prune(x))]
    else if std.isObject(a) then {
      [x]: $.prune(a[x])
      for x in std.objectFields(a)
      if isContent(std.prune(a[x]))
    } else
      a,

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
