use netem_trace::model::{BwTraceConfig, RepeatedBwPatternConfig};
use netem_trace::model::{NormalizedBwConfig, StaticBwConfig};
use netem_trace::{Bandwidth, Duration};
use netem_trace::{Mahimahi, MahimahiExt};

use polars::prelude::*;
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, Write};

use rayon::prelude::*;


use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// 封装的函数，处理指定目录的重命名和创建
fn backup_and_create_dir(dir_name: &str) -> std::io::Result<()> {
    // 获取当前时间戳
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    // 定义备份目录的名称
    let backup_dir_name = format!("{}_backup_{}", dir_name, timestamp);

    // 检查并重命名目录
    if Path::new(dir_name).exists() {
        fs::rename(dir_name, &backup_dir_name)?;
        println!("Renamed '{}' to '{}'", dir_name, backup_dir_name);
    }

    // 创建新的目录
    fs::create_dir(dir_name)?;
    println!("Created new directory '{}'", dir_name);

    Ok(())
}


fn main() -> Result<(), Box<dyn Error>> {

    backup_and_create_dir("mock_trace")?;
    backup_and_create_dir("mock_toml")?;


    let chosen = std::fs::File::open("choose.txt").map(
        |file: File| io::BufReader::new(file)
        .lines()
        .into_iter()
        .map(|line| line.unwrap())
        .collect::<HashSet<_>>()
    );


    let filter = |str: &String| {
        chosen.is_err() || chosen.as_ref().is_ok_and(|chosen| 
            chosen.contains(str.as_str().split_at(24).0))
    };


    // 读取 .parquet 文件
    let mut file = std::fs::File::open("mock.txt").unwrap();

    let reader = io::BufReader::new(file);

    let lines: Vec<String> = reader
        .lines()
        .into_iter()
        .map(|line| line.unwrap())
        .filter(filter)
        .collect();


    

    lines.par_iter().for_each(|line| {
        let split_line: Vec<String> = line
            .split_whitespace() // 按空格分割
            .map(String::from) // 转换为 String
            .collect(); // 收集到 Vec<String> 中

        assert_eq!(123, split_line.len());
        let mut split_line = split_line.into_iter();

        let file = split_line.next().unwrap();

        let delay = split_line.next().unwrap().parse::<f64>().unwrap();
        let loss = split_line.next().unwrap().parse::<f64>().unwrap();

        let tput = split_line
            .map(|value| value.as_str().parse::<f64>().unwrap())
            .collect::<Vec<f64>>();

        write(format!("{}_mock", file).as_str(), delay, loss, tput)
    });

    Ok(())
}

fn write(file: &str, delay: f64, loss: f64, tput: Vec<f64>) {
    let std_dev_options = [2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let queue_length_options = [100, 200];

    let mut traces = vec![];

    for std_dev in std_dev_options {
        let tput = tput
            .iter()
            .cloned()
            .map(|tput_seocnd| {
                // Box::new(
                //     StaticBwConfig::new()
                //         .bw(Bandwidth::from_kbps((tput_seocnd * 1000.0) as u64))
                //         .duration(Duration::from_millis(1000)),
                // ) as Box<dyn BwTraceConfig>
                Box::new(
                    NormalizedBwConfig::new()
                        .mean(Bandwidth::from_kbps((tput_seocnd * 1000.0) as u64))
                        .std_dev(Bandwidth::from_kbps(
                            (tput_seocnd * 1000.0 * std_dev) as u64,
                        ))
                        .duration(Duration::from_secs(1)),
                ) as Box<dyn BwTraceConfig>
            })
            .collect();

        let mut c = Box::new(RepeatedBwPatternConfig::new().pattern(tput).count(2)).into_model();

        c.mahimahi_to_file(
            &Duration::from_secs(120),
            format!("mock_trace/{}_{}.trace", file, std_dev),
        );

        for queue_len in queue_length_options {
            traces.push(format!(
                r#"
        {{ type = "BwReplay", trace = "./mock_trace/{}_{}.trace", queue = "DropTail", queue_config = {{ packet_limit = {} }} }},"#, file, std_dev, queue_len
            ))
        }
    }

    // 构造 TOML 配置字符串
    let toml_content = format!(
        r#"labels = ["NYC_mock0912"]
[network_set.uplink]
delay = [{{ type = "Delay", delay = "{}ms" }}]
bandwidth = [{{ type = "Bw", bandwidth = "100Mbps", queue = "Infinite" }}]
loss = [{{ type = "Loss", pattern = [] }}]

[network_set.downlink]
delay = [{{ type = "Delay", delay = "{}ms" }}]
bandwidth = [{}
]
loss = [{{ type = "Loss", pattern = [{:.5}] }}]
    "#,
        delay as u64,
        delay as u64,
        traces.join(""),
        loss
    );

    // 写入到文件
    let mut file = File::create(format!("mock_toml/{}.toml", file)).unwrap();
    file.write_all(toml_content.as_bytes()).unwrap();
}
