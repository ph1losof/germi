mod common;
use common::create_germi;
use germi::Error;

#[test]
fn test_nested_defaults() {
    let mut germi = create_germi();
    // Case: A unset, B unset, C set "final"
    germi.add_variable("C", "final");
    
    // ${A:-${B:-${C}}}
    let result = germi.interpolate("${A:-${B:-${C}}}").unwrap();
    assert_eq!(result, "final");
}

#[test]
fn test_nested_conditionals_and_defaults() {
    let mut germi = create_germi();
    germi.add_variable("HAS_VALUE", "yes");
    germi.add_variable("EMPTY", "");
    
    // If HAS_VALUE is set, result is "replacement".
    // Replacement is "${EMPTY:-fallback}".
    // EMPTY is set but empty. ":-" triggers on empty|unset.
    // So result should be "fallback".
    
    let result = germi.interpolate("${HAS_VALUE:+${EMPTY:-fallback}}").unwrap();
    assert_eq!(result, "fallback");
}

#[test]
fn test_deep_recursion_with_text() {
    let mut germi = create_germi();
    germi.add_variable("P1", "Part1");
    germi.add_variable("P2", "Part2");
    germi.add_variable("WRAPPER", "(${P1}|${P2})"); // (Part1|Part2)
    germi.add_variable("META", "[${WRAPPER}]");      // [(Part1|Part2)]
    
    let result = germi.interpolate("Result: ${META}").unwrap();
    assert_eq!(result, "Result: [(Part1|Part2)]");
}

#[test]
fn test_infinite_recursion_loop() {
    let mut germi = create_germi();
    germi.add_variable("LOOP", "${LOOP}");
    
    let result = germi.interpolate("${LOOP}");
    match result {
        Err(Error::RecursiveLookup(_)) => {},
        _ => panic!("Expected RecursiveLookup error, got {:?}", result),
    }
}

#[test]
fn test_complex_shell_like_structure() {
    let mut germi = create_germi();
    germi.add_variable("user", "admin");
    germi.add_variable("host", "localhost");
    germi.add_variable("port", "8080");
    germi.add_variable("path", "/var/www");
    germi.add_variable("def_path", "/tmp");
    
    // "scp ${user}@${host}:${path:-${def_path}}/file"
    // path is set, so uses /var/www
    let input = "scp ${user}@${host}:${path:-${def_path}}/file";
    let result = germi.interpolate(input).unwrap();
    assert_eq!(result, "scp admin@localhost:/var/www/file");
    
    // Now unset path
    // We can't unset in SimpleContext easily without exposing remove, but we can overwrite with empty string?
    // But default ":-" handles empty string too.
    germi.add_variable("path", "");
}

#[test]
fn test_triple_nested_default() {
    let mut germi = create_germi();
    germi.add_variable("D", "deepest");
    // ${A:-${B:-${C:-${D}}}}
    let res = germi.interpolate("${A:-${B:-${C:-${D}}}}").unwrap();
    assert_eq!(res, "deepest");
}

#[test]
fn test_mixed_nested_conditional_alternate() {
    let mut germi = create_germi();
    germi.add_variable("SET", "val");
    germi.add_variable("EMPTY", "");
    
    // ${SET:+${EMPTY-fallback}}
    // SET is val -> evaluate ${EMPTY-fallback}
    // EMPTY is "" -> loose alternate preserves empty -> ""
    // Result ""
    let res = germi.interpolate("${SET:+${EMPTY-fallback}}").unwrap();
    assert_eq!(res, "");

    // ${SET:+${EMPTY:-fallback}}
    // EMPTY is "" -> strict default uses fallback -> "fallback"
    // Result "fallback"
    let res2 = germi.interpolate("${SET:+${EMPTY:-fallback}}").unwrap();
    assert_eq!(res2, "fallback");
}

#[test]
fn test_massive_text_surround() {
    let germi = create_germi();
    let text_chunk = "0123456789".repeat(100); // 1000 chars
    let input = format!("{}${{TEST_VAR}}{}", text_chunk, text_chunk);
    let expected = format!("{}test_value{}", text_chunk, text_chunk);
    
    let res = germi.interpolate(&input).unwrap();
    assert_eq!(res, expected);
}

#[test]
fn test_lazy_evaluation_check() {
    let germi = create_germi();
    // ${TEST_VAR:+${MISSING}}
    // TEST_VAR is literal "test_value"
    // Wait, TEST_VAR is set. So it DOES evaluate ${MISSING}.
    // MISSING is missing -> Error.
    let res = germi.interpolate("${TEST_VAR:+${MISSING}}");
    assert!(matches!(res, Err(Error::MissingVar(_))));
    
    // ${MISSING:+${MISSING}}
    // Outer missing -> inner should NOT be evaluated?
    // Spec says: "If parameter is unset/null, null is substituted."
    // Does it evaluate "word"? Usually no.
    // If it did, it would error on ${MISSING} inside.
    let res2 = germi.interpolate("${MISSING:+${MISSING}}").unwrap();
    assert_eq!(res2, "");
}
