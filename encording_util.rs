include!("GenericError.rs");

use filename::file_name;
use std::fs::File;
use std::io::{ Read, Write, Seek };
use encoding_rs::SHIFT_JIS;

///　シフトJISでエンコードされたファイルを渡し、UTF8に変換して返す
/// 
/// inputパラメータ：シフトJISでエンコードされたファイル
pub fn sjis_to_uft8(mut input: &File) -> GenericResult<File> 
{
    //変換バッファ
    let mut s: Vec<u8> = Vec::new();

    //ファイル読み出し位置を最初に戻す
    match input.seek(std::io::SeekFrom::Start(0)) {
        Ok(_) => {},
        Err(e) => return Err(GenericError::from(e))
    }

    //ファイル全部のデータを読み込む
    match input.read_to_end(&mut s) {
        Ok(_) => {},
        Err(e) => return Err(GenericError::from(e))
    }

    // Shift_JISのバイト列(Vec<u8>) を UTF-8の文字列(&str) に変換
    let (res, _, _) = SHIFT_JIS.decode(&s);

    let text = res.into_owned();

    //ファイル名を読み出す
    let input_file_path = match file_name(input)
    {
        Ok(i) => i,
        Err(e) => return Err(GenericError::from(e))

    };

    //ファイルを作成
    let mut input: File = match File::create(input_file_path.clone()) { 
        Ok(f) => f,
        Err(e) => return Err(GenericError::from(e))
    };

    // 出力
    match input.write_all(text.as_bytes()) {
        Ok(_) => {},
        Err(e) => return Err(GenericError::from(e))        
    }

    //ファイルを読み出しモードにする
    match File::open(input_file_path) { 
        Ok(f) => return Ok(f),
        Err(e) => return Err(GenericError::from(e))
    };
}
