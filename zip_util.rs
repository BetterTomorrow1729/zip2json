include!("GenericError.rs");

use std::fs::File;
use std::io::{ Read, Write };
use std::path::Path;
use zip::ZipArchive;
use zip::read::ZipFile;

///zip圧縮されたファイルを指定されたパスに解凍する
/// 
/// zip_fileパラメータ：解凍したいzip圧縮されたファイル
/// 
/// savepathパラメータ：解凍したファイルを保存したいパス 
pub fn unzip_file(zip_file: &File, savepath: &Path) -> GenericResult<Vec<File>> {
    let mut result: Vec<File> = vec![];

    //zip圧縮されたファイルを開く
    let mut archive = match ZipArchive::new(zip_file) {
        Ok(a) => a,
        Err(e) => return Err(GenericError::from(e))
    };

    //格納されている各ファイルに対して処理する
    for i in 0..archive.len() {
        //格納されているファイルを開く
        let file: &mut ZipFile<'_> = &mut archive.by_index(i)?;

        //保存パスと格納ファイル名から保存ファイル名を決定する
        let out_item_name = savepath.join(file.name());

        //保存ファイルを作成する
        let mut outfile: File = match File::create(&out_item_name) { 
            Ok(f) => f,
            Err(e) => return Err(GenericError::from(e))
        };

        //格納ファイルからデータを読み出す
        let mut zip_data = vec![];
        match file.read_to_end(&mut zip_data) {
            Ok(_) => {},
            Err(e) => return Err(GenericError::from(e))
        }

        //保存ファイルにデータを書き込む
        match outfile.write_all(&zip_data) {
            Ok(_) => {},
            Err(e) => return Err(GenericError::from(e))
        };

        //保存ファイルを読み出しモードにし、戻り値に追加する
        match File::open(&out_item_name) { 
            Ok(f) => result.push(f),
            Err(e) => return Err(GenericError::from(e))
        };
    }

    Ok(result)
}
