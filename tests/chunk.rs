use std::{fs, io};

use mlua::{Chunk, ChunkMode, Lua, Result};

#[test]
fn test_chunk_methods() -> Result<()> {
    let lua = Lua::new();

    #[cfg(unix)]
    assert!(lua.load("return 123").name().contains("tests/chunk.rs"));
    let chunk2 = lua.load("return 123").set_name("@new_name");
    assert_eq!(chunk2.name(), "@new_name");

    let env = lua.create_table_from([("a", 987)])?;
    let chunk3 = lua.load("return a").set_environment(env.clone());
    assert_eq!(chunk3.environment().unwrap(), &env);
    assert_eq!(chunk3.mode(), ChunkMode::Text);
    assert_eq!(chunk3.call::<i32>(())?, 987);

    Ok(())
}

#[test]
fn test_chunk_path() -> Result<()> {
    let lua = Lua::new();

    if cfg!(target_arch = "wasm32") {
        // TODO: figure out why emscripten fails on file operations
        // Also see https://github.com/rust-lang/rust/issues/119250
        return Ok(());
    }

    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("module.lua"),
        r#"
        return 321
    "#,
    )?;
    let i: i32 = lua.load(temp_dir.path().join("module.lua")).eval()?;
    assert_eq!(i, 321);

    match lua.load(&*temp_dir.path().join("module2.lua")).exec() {
        Err(err) if err.downcast_ref::<io::Error>().unwrap().kind() == io::ErrorKind::NotFound => {}
        res => panic!("expected io::Error, got {:?}", res),
    };

    // &Path
    assert_eq!(
        (lua.load(&*temp_dir.path().join("module.lua").as_path())).eval::<i32>()?,
        321
    );

    Ok(())
}

#[test]
fn test_chunk_impls() -> Result<()> {
    let lua = Lua::new();

    // StdString
    assert_eq!(lua.load(String::from("1")).eval::<i32>()?, 1);
    assert_eq!(lua.load(&String::from("2")).eval::<i32>()?, 2);

    // &[u8]
    assert_eq!(lua.load(&b"3"[..]).eval::<i32>()?, 3);

    // Vec<u8>
    assert_eq!(lua.load(b"4".to_vec()).eval::<i32>()?, 4);
    assert_eq!(lua.load(&b"5".to_vec()).eval::<i32>()?, 5);

    Ok(())
}

#[test]
#[cfg(feature = "macros")]
fn test_chunk_macro() -> Result<()> {
    let lua = Lua::new();

    let name = "Rustacean";
    let table = vec![1];

    let data = lua.create_table()?;
    data.raw_set("num", 1)?;

    let ud = mlua::AnyUserData::wrap("hello");
    let f = mlua::Function::wrap(|| Ok(()));

    lua.globals().set("g", 123)?;

    let string = String::new();
    let str = string.as_str();

    lua.load(mlua::chunk! {
        assert($name == "Rustacean")
        assert(type($table) == "table")
        assert($table[1] == 1)
        assert(type($data) == "table")
        assert($data.num == 1)
        assert(type($ud) == "userdata")
        assert(type($f) == "function")
        assert(type($str) == "string")
        assert($str == "")
        assert(g == 123)
        s = 321
    })
    .exec()?;

    assert_eq!(lua.globals().get::<i32>("s")?, 321);

    Ok(())
}

#[cfg(feature = "luau")]
#[test]
fn test_compiler() -> Result<()> {
    use std::vec;

    let compiler = mlua::Compiler::new()
        .set_optimization_level(2)
        .set_debug_level(2)
        .set_type_info_level(1)
        .set_coverage_level(2)
        .set_vector_lib("vector")
        .set_vector_ctor("new")
        .set_vector_type("vector")
        .set_mutable_globals(vec!["mutable_global".into()])
        .set_userdata_types(vec!["MyUserdata".into()]);

    assert!(compiler.compile("return vector.new(1, 2, 3)").is_ok());

    // Error
    match compiler.compile("%") {
        Err(mlua::Error::SyntaxError { ref message, .. }) => {
            assert!(message.contains("Expected identifier when parsing expression, got '%'"),);
        }
        res => panic!("expected result: {res:?}"),
    }

    Ok(())
}

#[test]
fn test_chunk_wrap() -> Result<()> {
    let lua = Lua::new();

    let f = Chunk::wrap("return 123");
    lua.globals().set("f", f)?;
    lua.load("assert(f() == 123)").exec().unwrap();

    lua.globals().set("f2", Chunk::wrap("c()"))?;
    assert!(
        (lua.load("f2()").exec().err().unwrap().to_string()).contains(file!()),
        "wrong chunk location"
    );

    Ok(())
}
