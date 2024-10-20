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

  split(str, c)::
    assert std.isString(str) : 'std.split first parameter must be a String, got ' + std.type(str);
    assert std.isString(c) : 'std.split second parameter must be a String, got ' + std.type(c);
    assert std.length(c) >= 1 : 'std.split second parameter must have length 1 or greater, got ' + std.length(c);
    std.splitLimit(str, c, -1),

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

  manifestYamlDoc(value, indent_array_in_object=false, quote_keys=true)::
    local onlyChars(charSet, strSet) =
      if std.length(std.setInter(charSet, strSet)) == std.length(strSet) then
        true
      else false;
    local isReserved(key) =
      // NOTE: These values are checked for case insensitively.
      // While this approach results in some false positives, it eliminates
      // the risk of missing a permutation.
      local reserved = [
        // Boolean types taken from https://yaml.org/type/bool.html
        'true',
        'false',
        'yes',
        'no',
        'on',
        'off',
        'y',
        'n',
        // Numerical words taken from https://yaml.org/type/float.html
        '.nan',
        '-.inf',
        '+.inf',
        '.inf',
        'null',
        // Invalid keys that contain no invalid characters
        '-',
        '---',
        '',
      ];
      local bad = [word for word in reserved if word == std.asciiLower(key)];
      if std.length(bad) > 0 then
        true
      else false;
    local typeMatch(m_key, type) =
      // Look for positive or negative numerical types (ex: 0x)
      if std.substr(m_key, 0, 2) == type || std.substr(m_key, 0, 3) == '-' + type then
        true
      else false;
    local bareSafe(key) =
      /*
      For a key to be considered safe to emit without quotes, the following must be true
        - All characters must match [a-zA-Z0-9_/\-]
        - Not match the integer format defined in https://yaml.org/type/int.html
        - Not match the float format defined in https://yaml.org/type/float.html
        - Not match the timestamp format defined in https://yaml.org/type/timestamp.html
        - Not match the boolean format defined in https://yaml.org/type/bool.html
        - Not match the null format defined in https://yaml.org/type/null.html
        - Not match (ignoring case) any reserved words which pass the above tests.
          Reserved words are defined in isReserved() above.

      Since the remaining YAML types require characters outside the set chosen as valid
      for the elimination of quotes from the YAML output, the remaining types listed at
      https://yaml.org/type/ are by default always quoted.
      */
      local letters = std.set(std.stringChars('ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_-/'));
      local digits = std.set(std.stringChars('0123456789'));
      local intChars = std.set(digits + std.stringChars('_-'));
      local binChars = std.set(intChars + std.stringChars('b'));
      local hexChars = std.set(digits + std.stringChars('abcdefx_-'));
      local floatChars = std.set(digits + std.stringChars('e._-'));
      local dateChars = std.set(digits + std.stringChars('-'));
      local safeChars = std.set(letters + floatChars);
      local keyLc = std.asciiLower(key);
      local keyChars = std.stringChars(key);
      local keySet = std.set(keyChars);
      local keySetLc = std.set(std.stringChars(keyLc));
      // Check for unsafe characters
      if !onlyChars(safeChars, keySet) then
        false
      // Check for reserved words
      else if isReserved(key) then
        false
      /* Check for timestamp values.  Since spaces and colons are already forbidden,
         all that could potentially pass is the standard date format (ex MM-DD-YYYY, YYYY-DD-MM, etc).
         This check is even more conservative: Keys that meet all of the following:
           - all characters match [0-9\-]
           - has exactly 2 dashes
         are considered dates.
      */
      else if onlyChars(dateChars, keySet)
              && std.length(std.findSubstr('-', key)) == 2 then
        false
      /* Check for integers.  Keys that meet all of the following:
           - all characters match [0-9_\-]
           - has at most 1 dash
         are considered integers.
      */
      else if onlyChars(intChars, keySetLc)
              && std.length(std.findSubstr('-', key)) < 2 then
        false
      /* Check for binary integers.  Keys that meet all of the following:
           - all characters match [0-9b_\-]
           - has at least 3 characters
           - starts with (-)0b
         are considered binary integers.
      */
      else if onlyChars(binChars, keySetLc)
              && std.length(key) > 2
              && typeMatch(key, '0b') then
        false
      /* Check for floats. Keys that meet all of the following:
           - all characters match [0-9e._\-]
           - has at most a single period
           - has at most two dashes
           - has at most 1 'e'
         are considered floats.
      */
      else if onlyChars(floatChars, keySetLc)
              && std.length(std.findSubstr('.', key)) == 1
              && std.length(std.findSubstr('-', key)) < 3
              && std.length(std.findSubstr('e', keyLc)) < 2 then
        false
      /* Check for hexadecimals.  Keys that meet all of the following:
           - all characters match [0-9a-fx_\-]
           - has at most 1 dash
           - has at least 3 characters
           - starts with (-)0x
         are considered hexadecimals.
      */
      else if onlyChars(hexChars, keySetLc)
              && std.length(std.findSubstr('-', key)) < 2
              && std.length(keyChars) > 2
              && typeMatch(key, '0x') then
        false
      // All checks pass. Key is safe for emission without quotes.
      else true;
    local escapeKeyYaml(key) =
      if bareSafe(key) then key else std.escapeStringJson(key);
    local aux(v, path, cindent) =
      if v == true then
        'true'
      else if v == false then
        'false'
      else if v == null then
        'null'
      else if std.isNumber(v) then
        '' + v
      else if std.isString(v) then
        local len = std.length(v);
        if len == 0 then
          '""'
        else if v[len - 1] == '\n' then
          local split = std.split(v, '\n');
          std.join('\n' + cindent + '  ', ['|'] + split[0:std.length(split) - 1])
        else
          std.escapeStringJson(v)
      else if std.isFunction(v) then
        error 'Tried to manifest function at ' + path
      else if std.isArray(v) then
        if std.length(v) == 0 then
          '[]'
        else
          local params(value) =
            if std.isArray(value) && std.length(value) > 0 then {
              // While we could avoid the new line, it yields YAML that is
              // hard to read, e.g.:
              // - - - 1
              //     - 2
              //   - - 3
              //     - 4
              new_indent: cindent + '  ',
              space: '\n' + self.new_indent,
            } else if std.isObject(value) && std.length(value) > 0 then {
              new_indent: cindent + '  ',
              // In this case we can start on the same line as the - because the indentation
              // matches up then.  The converse is not true, because fields are not always
              // 1 character long.
              space: ' ',
            } else {
              // In this case, new_indent is only used in the case of multi-line strings.
              new_indent: cindent,
              space: ' ',
            };
          local range = std.range(0, std.length(v) - 1);
          local parts = [
            '-' + param.space + aux(v[i], path + [i], param.new_indent)
            for i in range
            for param in [params(v[i])]
          ];
          std.join('\n' + cindent, parts)
      else if std.isObject(v) then
        if std.length(v) == 0 then
          '{}'
        else
          local params(value) =
            if std.isArray(value) && std.length(value) > 0 then {
              // Not indenting allows e.g.
              // ports:
              // - 80
              // instead of
              // ports:
              //   - 80
              new_indent: if indent_array_in_object then cindent + '  ' else cindent,
              space: '\n' + self.new_indent,
            } else if std.isObject(value) && std.length(value) > 0 then {
              new_indent: cindent + '  ',
              space: '\n' + self.new_indent,
            } else {
              // In this case, new_indent is only used in the case of multi-line strings.
              new_indent: cindent,
              space: ' ',
            };
          local lines = [
            (if quote_keys then std.escapeStringJson(k) else escapeKeyYaml(k)) + ':' + param.space + aux(v[k], path + [k], param.new_indent)
            for k in std.objectFields(v)
            for param in [params(v[k])]
          ];
          std.join('\n' + cindent, lines);
    aux(value, [], ''),

  manifestYamlStream(value, indent_array_in_object=false, c_document_end=true, quote_keys=true)::
    if !std.isArray(value) then
      error 'manifestYamlStream only takes arrays, got ' + std.type(value)
    else
      '---\n' + std.join(
        '\n---\n', [std.manifestYamlDoc(e, indent_array_in_object, quote_keys) for e in value]
      ) + if c_document_end then '\n...\n' else '\n',


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

  local base64_table = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/',
  local base64_inv = { [base64_table[i]]: i for i in std.range(0, 63) },

  base64(input)::
    local bytes =
      if std.isString(input) then
        std.map(std.codepoint, input)
      else
        input;

    local aux(arr, i, r) =
      if i >= std.length(arr) then
        r
      else if i + 1 >= std.length(arr) then
        local str =
          // 6 MSB of i
          base64_table[(arr[i] & 252) >> 2] +
          // 2 LSB of i
          base64_table[(arr[i] & 3) << 4] +
          '==';
        aux(arr, i + 3, r + str) tailstrict
      else if i + 2 >= std.length(arr) then
        local str =
          // 6 MSB of i
          base64_table[(arr[i] & 252) >> 2] +
          // 2 LSB of i, 4 MSB of i+1
          base64_table[(arr[i] & 3) << 4 | (arr[i + 1] & 240) >> 4] +
          // 4 LSB of i+1
          base64_table[(arr[i + 1] & 15) << 2] +
          '=';
        aux(arr, i + 3, r + str) tailstrict
      else
        local str =
          // 6 MSB of i
          base64_table[(arr[i] & 252) >> 2] +
          // 2 LSB of i, 4 MSB of i+1
          base64_table[(arr[i] & 3) << 4 | (arr[i + 1] & 240) >> 4] +
          // 4 LSB of i+1, 2 MSB of i+2
          base64_table[(arr[i + 1] & 15) << 2 | (arr[i + 2] & 192) >> 6] +
          // 6 LSB of i+2
          base64_table[(arr[i + 2] & 63)];
        aux(arr, i + 3, r + str) tailstrict;

    local sanity = std.all([a < 256 for a in bytes]);
    if !sanity then
      error 'Can only base64 encode strings / arrays of single bytes.'
    else
      aux(bytes, 0, ''),


  base64DecodeBytes(str)::
    if std.length(str) % 4 != 0 then
      error 'Not a base64 encoded string "%s"' % str
    else
      local aux(str, i, r) =
        if i >= std.length(str) then
          r
        else
          // all 6 bits of i, 2 MSB of i+1
          local n1 = [base64_inv[str[i]] << 2 | (base64_inv[str[i + 1]] >> 4)];
          // 4 LSB of i+1, 4MSB of i+2
          local n2 =
            if str[i + 2] == '=' then []
            else [(base64_inv[str[i + 1]] & 15) << 4 | (base64_inv[str[i + 2]] >> 2)];
          // 2 LSB of i+2, all 6 bits of i+3
          local n3 =
            if str[i + 3] == '=' then []
            else [(base64_inv[str[i + 2]] & 3) << 6 | base64_inv[str[i + 3]]];
          aux(str, i + 4, r + n1 + n2 + n3) tailstrict;
      aux(str, 0, []),

  base64Decode(str)::
    local bytes = std.base64DecodeBytes(str);
    std.join('', std.map(std.char, bytes)),

  reverse(arr)::
    local l = std.length(arr);
    std.makeArray(l, function(i) arr[l - i - 1]),

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

  find(value, arr)::
    if !std.isArray(arr) then
      error 'find second parameter should be an array, got ' + std.type(arr)
    else
      std.filter(function(i) arr[i] == value, std.range(0, std.length(arr) - 1)),

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
