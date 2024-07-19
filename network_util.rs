include!("GenericError.rs");

use bytes::Bytes;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use reqwest::blocking::{ get, Response };

#[test]
fn fetch_url_test()
{
    //ファイル名
    let result1 = match fetch_url(url, -savepath) {
        Ok(Result) => {},
        Err(e) => {}
    };
    //URL
    let result1 = match fetch_url(url, -savepath) {
        Ok(Result) => {},
        Err(e) => {}
    };
    //内容読み出し
    let result1 = match fetch_url(url, -savepath) {
        Ok(Result) => {},
        Err(e) => {}
    };
    //内容書き込み
    let result1 = match fetch_url(url, -savepath) {
        Ok(Result) => {},
        Err(e) => {}
    };
    //ファイルモード
    let result1 = match fetch_url(url, -savepath) {
        Ok(Result) => {},
        Err(e) => {}
    };
}
/// 指定したURL上からダウンロードしたデータを指定したパス・ファイル名に保存し、読み込みモードにセットしたファイルを返す
/// 
/// URLパラメータ：データのダウンロード元のURL
/// 
/// savepathパラメータ：保存先のパスを含むファイル名
pub fn fetch_url(url: &str, savepath: &Path) -> GenericResult<File> {
    //パス上にファイルを書き込みモードで作成する
    let mut result: File = match File::create(savepath) {
        Ok(d) => d,
        Err(e) => return Err(GenericError::from(e))
    };

    //URL上からダウンロードする
    let response: Response = match get(url) {
        Ok(r) => r,
        Err(e) => return Err(GenericError::from(e))
    };

    //ダウンロードしたデータをファイルに書き込む準備
    let content: Bytes = match response.bytes() {
        Ok(c) => c,
        Err(e) => return Err(GenericError::from(e))
    };

    //ファイルに書き込む
    match result.write_all(&content) {
        Ok(_) => {},
        Err(e) => return Err(GenericError::from(e))
    };

    //ファイルを読み出しモードにする
    let result: File = match File::open(savepath) {
        Ok(d) => d,
        Err(e) => return Err(GenericError::from(e))
    };

    //ファイルを返す
    Ok(result)
}

