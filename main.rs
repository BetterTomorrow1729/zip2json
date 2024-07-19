use serde::Serialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::io::{ Read, Seek };
use std::path::Path;
use tempdir::TempDir;

include!("GenericError.rs");

mod encording_util;
mod network_util;
mod zip_util;

/// 処理しているのが全件郵便番号ファイルであるか個別事業所郵便番号ファイルであるかを示す
enum CSVType {
    CsvtKenAll,
    CsvtJigyosho
}

#[derive(PartialEq)]
enum CommandLineParam { 
    SavePathMode (OsString),
    UsageMode,
    ParameterError
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
/// 郵便番号の下4桁一件のデータを保持する
/// 
/// codeフィールド：都道府県・自治体テーブルのインデックス
/// 
/// addressフィールド：町域（全件郵便番号）・町域/番地/事業所名(個別事業所郵便番号)を保持する
/// 
/// 一つの郵便番号下４桁に対して複数のデータがぶら下がっていることが稀にあるため二次元ベクタになっている
struct OneLowerZipStore
{
    code: i32,
    address: Vec<Vec<String>>
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
/// 都道府県・自治体の組み合わせを保持する
/// 
/// prefフィールド：都道府県
/// 
/// municフィールド：自治体
struct PrefAndMunic
{
    pref:String,
    munic:String
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
/// 郵便番号上3桁一つ分のデータを保持する
/// 
/// pref_and_municsフィールド：この郵便番号上3桁の中にある都道府県・自治体のリスト
/// 
/// addressフィールド：この郵便番号上3桁にぶら下がっている下4桁郵便番号と住所のリスト
struct OneUpperZipStore 
{
    pref_and_munics: HashMap<i32, PrefAndMunic>,
    address: HashMap<String, OneLowerZipStore>
}

impl OneUpperZipStore {
    fn new() -> Self
    {
        OneUpperZipStore { pref_and_munics : HashMap::new(), address : HashMap::new() }
    }
}

/// 郵便番号データが格納されているCSVファイルを処理し、郵便番号上3桁をキーとしたHashMapに格納する
/// 
/// type selectorパラメータ：全件郵便番号か個別事業所郵便番号のどちらのデータであるかを指定する
/// 
/// filesパラメータ：csvデータが格納されているファイル列
/// 
/// resultパラメータ：処理結果を格納する
fn process_csv_files(type_seletor: CSVType, files: &Vec<File>, result: &mut HashMap<String, OneUpperZipStore>) -> GenericResult<()>
{
    // 処理バッファ
    let mut buf = String::new();

    for mut file in files {
        // ファイル読み出し位置を最初に戻す
        file.rewind().unwrap();

        // ファイル全体を読み出す
        file.read_to_string(&mut buf).unwrap();

        // 郵便番号の上3桁と下4桁を保持する
        let mut upper_zipcode: String;
        let mut lower_zipcode: String;

        // 各行を処理する
        for line in buf.lines() {
            // ’，’で区切って配列化する
            let columns:Vec<&str> = line.split(',').collect(); 

            // 郵便番号、都道府県、自治体、町域、番地、事業所名を配列から読み込む
            // データの形式については以下のURLを参照
            // 全件郵便番号データ：https://www.post.japanpost.jp/zipcode/dl/readme.html
            // 個別事業所郵便番号データ：https://www.post.japanpost.jp/zipcode/dl/jigyosyo/readme.html
            let (mut zip_code, mut prefecture, mut municipalitie, mut town_area, mut house_number, mut jigyosho) =  
            match type_seletor {
                CSVType::CsvtKenAll => (
                    columns[2].to_string(),
                    columns[6].to_string(),
                    columns[7].to_string(),
                    columns[8].to_string(),
                    "".to_string(),
                    "".to_string()
                ),
                CSVType::CsvtJigyosho => (
                    columns[7].to_string(),
                    columns[3].to_string(),
                    columns[4].to_string(),
                    columns[5].to_string(),
                    columns[6].to_string(),
                    columns[2].to_string()
                )
            };

            // 各データの前後についている引用符を取り除く
            zip_code = zip_code.replace("\"", "");
            prefecture = prefecture.replace("\"", "");
            municipalitie = municipalitie.replace("\"", "");
            town_area = town_area.replace("\"", "");
            house_number = house_number.replace("\"", "");
            jigyosho = jigyosho.replace("\"", "");

            // "以下に掲載がない場合"はカットする
            if town_area.eq("以下に掲載がない場合") { town_area = "".to_string(); };

            // 括弧で挟まれた部分の処理(その他・次のビルを除く・地階・階層不明と括弧をカット)
            for pat in vec!["（その他）", "（次のビルを除く）", "（地階・階層不明）","（", "）"] {
                town_area = town_area.replace(pat, "");
            }

            // 郵便番号の上3桁と下4桁を取り出す
            upper_zipcode = zip_code[..3].to_string();
            lower_zipcode = zip_code[3..].to_string();

            // 上3桁一つ分のデータを取得する。未登録の場合は新規登録する。
            let upper_zip_store = result.entry(upper_zipcode).or_insert(OneUpperZipStore::new());

            // 一致する都道府県・自治体を検索しそのコードを求める。未登録の場合は新しいコードで新規登録する。
            let mut found = false;
            let mut p_m_id = 1;
            for (code, p_m) in &upper_zip_store.pref_and_munics {
                if p_m.pref == prefecture && p_m.munic == municipalitie {
                    found = true;
                    p_m_id = *code;
                    break;
                };
                p_m_id += 1;
            };
            if !found {
                upper_zip_store.pref_and_munics.insert(p_m_id, PrefAndMunic { pref: prefecture, munic: municipalitie });    
            };

            // 下4桁のデータを取得する。未登録の場合は新規登録する。
            let lower_zip_store = upper_zip_store.address.entry(lower_zipcode).or_insert(OneLowerZipStore { code: p_m_id, address: vec![] });

            // 全件郵便番号なら町域、個別事業所郵便番号なら町域/番地/事業所名を登録する。
            let item =  
            match type_seletor {
                CSVType::CsvtKenAll => vec![town_area],
                CSVType::CsvtJigyosho => vec![town_area, house_number, jigyosho]
            };
            lower_zip_store.address.append(&mut vec![item]);
        };
    };

    Ok(())
}

/// 全件郵便番号処理
fn process_ken_all_zipdata(work_path: &Path, entire_zip_data: &mut HashMap<String, OneUpperZipStore>) -> bool 
{
    // 作業用ファイル名をセット
    let kenall_filename = work_path.join("ken_all.zip");

    // 郵政省サイトより全件郵便番号圧縮Zipファイルをダウンロード    
    let kenall = match network_util::fetch_url("https://www.post.japanpost.jp/zipcode/dl/kogaki/zip/ken_all.zip", &kenall_filename)
    {
        Ok(file) => {
            println!("全件郵便番号読み込み完了。"); 
            file
        },
        Err(_) => {
            eprintln!("全件郵便番号読み込み時にエラーが発生しました。");
            return false
        }
    };

    // 全件郵便番号圧縮Zipファイルを解凍
    let kenall_files = match zip_util::unzip_file(&kenall, work_path) {
        Ok(files) => {
            println!("全件郵便番号ファイル解凍完了。");
            files
        },
        Err(_) => {
            eprintln!("全件郵便番号ファイル解凍時にエラーが発生しました。");
            return false
        }
    };

    // ShiftJISエンコードになっているのでUTF8エンコードにする
    for kenall_file in &kenall_files {
        match encording_util::sjis_to_uft8(kenall_file) {
          Ok(_) => {
            println!("全件郵便番号文字コード変換完了。");
          },
          Err(_) => {
            eprintln!("全件郵便番号文字コード変換時にエラーが発生しました。");
            return false  
          }
        };            
    }
    
    // 全件郵便番号を読み込んで内部データに書き込む
    match process_csv_files(CSVType::CsvtKenAll, &kenall_files, entire_zip_data) {
        Ok(_) => {
            println!("全件郵便番号データ処理完了。");
        },
        Err(_) => {
            eprintln!("全件郵便番号データ処理時にエラーが発生しました。");
            return false
        }
    };

    println!("全件郵便番号処理完了。");

    true
}

/// 個別事業所郵便番号処理
fn process_jigyosyo_zipdata(work_path: &Path, entire_zip_data: &mut HashMap<String, OneUpperZipStore>) -> bool 
{
    // 作業用ファイル名をセット
    let jigyosyo_filename = work_path.join("jigyosyo.zip");

    // 郵政省サイトより個別事業所郵便番号圧縮Zipファイルをダウンロード    
    let jigyosyo = match network_util::fetch_url("https://www.post.japanpost.jp/zipcode/dl/jigyosyo/zip/jigyosyo.zip", &jigyosyo_filename) {
        Ok(file) => {
            println!("大口個別事業者郵便番号読み込み完了。");
            file
        },
        Err(_) => {
            eprintln!("大口個別事業者郵便番号読み込み時にエラーが発生しました。");
            return false               
        }
    };

    // 個別事業所郵便番号圧縮Zipファイルを解凍
    let jigyosyo_files = match zip_util::unzip_file(&jigyosyo, work_path) {
        Ok(files) => {
            println!("大口個別事業者郵便番号ファイル解凍完了。");
            files
        },
        Err(_) => {
            eprintln!("大口個別事業者郵便番号ファイル解凍時にエラーが発生しました。");
            return false
        }
    };

    // ShiftJISエンコードになっているのでUTF8エンコードにする
    for jigyosyo_file in &jigyosyo_files {
        match encording_util::sjis_to_uft8(jigyosyo_file) {
            Ok(_) => {
                println!("大口個別事業者郵便番号文字コード変換完了。");
            },
            Err(_) => {
                eprintln!("大口個別事業者郵便番号文字コード変換時にエラーが発生しました。");
                return false  
            }
          };            
    }

    // 個別事業所郵便番号を読み込んで内部データに書き込む
    match process_csv_files(CSVType::CsvtJigyosho, &jigyosyo_files, entire_zip_data) {
        Ok(_) => {
            println!("大口個別事業者郵便番号データ処理完了。");
        },
        Err(_) => {
            eprintln!("大口個別事業者郵便番号データ処理時にエラーが発生しました。");
            return false
        }
    };

    println!("大口個別事業者郵便番号処理完了。");

    true
}

// コマンドラインパラメータを解析し、処理する
fn parameter_check() -> CommandLineParam
{
    // 受け入れられる場合以外は全てエラーとする
    let mut result:CommandLineParam = CommandLineParam::ParameterError;

    // パラメータを配列化する
    let args:Vec<OsString> = std::env::args_os().collect();

    // パラメータ個数で処理分け
    match args.len() 
    {
        // パラメータ無しの場合はカレントディレクトリに保存する
        1 => {
            result = CommandLineParam::SavePathMode(".".into());
        }
        // パラメータ一個の場合はヘルプオプション以外は全てエラー
        2 => {
            match args[1].to_str() {
                Some("-h") => {
                    result = CommandLineParam::UsageMode;
                },
                None | Some(&_) => {}
            }
        }
        // パラメータ一個の場合は保存パス指定オプション以外は全てエラー
        // 指定したパスが無ければ作成する
        3 => {
            if args[1] == "-path" {
                let path = Path::new(&args[2]);
                let mut path_exist = Path::is_dir(path);
                if !path_exist {
                    match fs::create_dir_all(path) {
                        Ok(_) => {
                            path_exist = true;
                        },
                        Err(_) => {}
                    }
                }

                if path_exist {
                    result = CommandLineParam::SavePathMode(path.into());
                }
            }
        },
        // それ以外は全てエラー
        _ => {}
    }

    result
}

fn main() 
{
    let save_path: OsString;

    // コマンドライン引数をパースする
    match parameter_check() {
        CommandLineParam::SavePathMode(path) => {
            save_path = path;
        },
        CommandLineParam::UsageMode | CommandLineParam::ParameterError => {
            println!("Usage: zip2json [-path ZipdataSavePath] | [ -h ]");
            std::process::exit(0);
        }
    };

    // 郵便番号読み込みバッファ
    let mut entire_zip_data: HashMap<String, OneUpperZipStore> = HashMap::new();

    // テンポラリパスを求める
    let binding = TempDir::new("zip").unwrap();
    let temp_path = binding.path();

    // 全件郵便番号を処理する
    if !process_ken_all_zipdata(temp_path, &mut entire_zip_data) {
        return;
    }

    // 個別事業所郵便番号を処理する
    if !process_jigyosyo_zipdata(temp_path, &mut entire_zip_data) {
        return;
    }

    // 郵便番号上3桁ごとに一つのjsonファイルを作成して保存する
    for zip_number in 1..1000 {
        let upper_zip_code = format!("{:03}", zip_number);

        match entire_zip_data.get(&upper_zip_code) {
            Some(val) => {
                // jsonファイルを作成する
                let file = match File::create(Path::new(&save_path).join(format!("{:03}.json", upper_zip_code))) {
                    Ok(f) => f,
                    Err(_) => {
                        eprintln!("{:03}.JSONファイルの作成に失敗しました", upper_zip_code);
                        std::process::exit(1);
                    }
                };

                // jsonファイルに書き込む
                match serde_json::to_writer_pretty(file, &val) {
                    Ok(_) => {},
                    Err(err) => {
                        eprintln!("JSONファイルへの保存に失敗しました: {}", err);
                        std::process::exit(1);
                    }
                }

                // 処理進行インジケータ表示
                print!(" {:03}", upper_zip_code);
            },
            None => { print!("    "); }
        }

        // 10個ごとに改行
        if zip_number % 10 == 0 { println!("") };
    }

    // 改行
    println!("");
    
    println!("全郵便番号データ処理完了。");
}
