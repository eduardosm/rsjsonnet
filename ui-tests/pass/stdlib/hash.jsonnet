std.assertEqual(std.md5(""), "d41d8cd98f00b204e9800998ecf8427e") &&
std.assertEqual(std.md5("hello world"), "5eb63bbbe01eeed093cb22bb8f5acdc3") &&

std.assertEqual(std.sha1(""), "da39a3ee5e6b4b0d3255bfef95601890afd80709") &&
std.assertEqual(std.sha1("hello world"), "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed") &&

std.assertEqual(std.sha256(""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855") &&
std.assertEqual(std.sha256("hello world"), "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9") &&

std.assertEqual(
  std.sha512(""),
  "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
) &&
std.assertEqual(
  std.sha512("hello world"),
  "309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f",
) &&

std.assertEqual(
  std.sha3(""),
  "a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26",
) &&
std.assertEqual(
  std.sha3("hello world"),
  "840006653e9ac9e95117a15c915caab81662918e925de9e004f774ff82d7079a40d4d27b1b372657c61d46d470304c88c788b3a4527ad074d1dccbee5dbaa99a",
) &&

true
