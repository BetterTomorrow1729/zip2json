/// 一つの関数で複数のカテゴリのエラーが起こりうる場合それを一つの型で表すための表現
/// 
/// 参照：オライリージャパン　プログラミングRust 第２版　pp.146
pub type GenericError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// GenericErrorを受けるResult型
pub type GenericResult<T> = Result<T, GenericError>;
