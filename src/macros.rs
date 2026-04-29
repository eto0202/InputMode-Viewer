/// 失敗したらcontinueする (Result)
#[macro_export]
macro_rules! skip_err {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(_) => continue,
        }
    };
}

/// 失敗したらcontinueする (Option)
#[macro_export]
macro_rules! skip_none {
    ($opt:expr) => {
        match $opt {
            Some(val) => val,
            None => continue,
        }
    };
}

#[macro_export]
/// 改行対策
/// 値がなければ何も返さず return (関数の戻り値が () の場合)
macro_rules! guard_opt {
    ($e:expr) => {
        match $e {
            Some(v) => v,
            None => return,
        }
    };
    ($e:expr, $else_ret:expr) => {
        match $e {
            Some(v) => v,
            None => return $else_ret,
        }
    };
}

#[macro_export]
/// 改行対策
/// エラーなら return
macro_rules! guard_res {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(_) => return,
        }
    };
}
