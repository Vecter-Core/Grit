### G08_probes — [bản sao] ~110 chương trình THỬ TẤN CÔNG tự viết (audit/probes): tìm âm tính giả

- **Trạng thái:** FAIL (mã thoát 1, 4s, bắt đầu 2026-10-20T12:10:15Z)
- 181 probe tự viết; FALSE_NEGATIVE=23; KNOWN_GAP=3; OK=155

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
FALSE_NEGATIVE  m6_same_stmt_call_args_double_move            kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_cap_duplicated                   kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_generic_dup                      kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_method_self_and_arg              kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_struct_lit_double_move           kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_tuple_double_move                kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_tuple_use_after_move             kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_unique_ref_duplicated            kỳ vọng=reject E0401                 verdict=green E=[] S=[]
FALSE_NEGATIVE  m6_same_stmt_use_after_move_in_args           kỳ vọng=reject E0401                 verdict=green E=[] S=[]
KNOWN_GAP       secret_evasion_split_consts                   kỳ vọng=warn S101                    verdict=green E=[] S=[]
KNOWN_GAP       secret_evasion_unicode_escape                 kỳ vọng=warn S101                    verdict=green E=[] S=[]
FALSE_NEGATIVE  ty_alias_cycle                                kỳ vọng=reject                       verdict=green E=[] S=[]
FALSE_NEGATIVE  type_recursive_enum                           kỳ vọng=reject                       verdict=green E=[] S=[]
FALSE_NEGATIVE  type_recursive_struct                         kỳ vọng=reject                       verdict=green E=[] S=[]
FALSE_NEGATIVE  type_recursive_struct_used                    kỳ vọng=reject                       verdict=green E=[] S=[]
FALSE_NEGATIVE  type_slice_field_by_value                     kỳ vọng=reject E0301                 verdict=green E=[] S=[]
FALSE_NEGATIVE  type_slice_param_by_value                     kỳ vọng=reject E0301                 verdict=green E=[] S=[]
FALSE_NEGATIVE  type_str_param_by_value                       kỳ vọng=reject E0301                 verdict=green E=[] S=[]
```
</details>
