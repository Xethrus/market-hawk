use anyhow::{Context, Result};
use anyhow::anyhow;
use curl::easy::Easy;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

use statrs::statistics::Statistics;
use std::str;
use std::collections::HashMap;

use config::ConfigError;
use config::{Config, File, FileFormat};
use std::io::Write;

struct StockData {
    symbol: String,
    basic_metrics: BasicMetrics,
    time_period: i16,
    daily_states: Vec<DailyStockData>,
}

struct DailyStockData {
    closing_price: f64,
    volume: f64,
    change_in_value: f64,
}

struct BasicMetrics {
    mean_return: f64,
    mean_value: f64,
    mean_volume: f64,
    variance: f64,
    standard_deviation: f64,
}

//dont know what is needed here
//figure out how this works
#[derive(PartialEq<_>,Debug, Deserialize)]
enum Source {
    api, 
    local,
}

struct SymbolData {
    data: serde_json::Value,
    //maybe could denote which symbols were grabbable, this would be good for the api limit
    symbols: Vec<String>,
    source: Source,
    time_period: i16,
}


#[derive(Debug, Deserialize)]
struct ClientConfig {
    source: Source,
    file_path: String,
    api_key: String,
    symbols: Vec<String>,
    time_period: i16,
}


//fn make_fapi_request() -> Result<serde_json::Value> {
//    let file = File::open(testing_load.json)?;
//        let json_data: Value = serde_json::from_reader(file)?;
//        Ok(json_data)
//}

//TODO REHAUL DATA GATHERING
//TODO WHAT I NEED TO DO IS MAKE IT SO THAT API DATA COMES WITH ALL THE SYMBOLS AND SUCH AND THAT
//THE LOCAL DATA DOES THE SAME, BASICALLY NEED TO FIGURE A BUNCH OF GENERAL SHIT ABOUT THAT
//STRUCTURING AND DATA MODEL: SINCE API MAKES A CALL AND GET SOME DATA, BUT IN THEORY THE LOCAL
//DATA IS GRABBED ALL AT ONCE THEN  "PARSED" FOR WANTED SYMBOLS hmmmmmmmmmmm
//
//CHANGE API SO IT RUNS MULTIPLE CYCLES SO IT HAS DATA LIKE LOCAL, MAXIMIZE EFFICIENCY
//
//
//TWO FUNCTIONS RETURN NORMALIZED DATA, DOING TOO MUCH


fn grab_client_config() -> Result<ClientConfig, ConfigError> {
    let configuration = Config::default();
    let configuration = Config::builder()
        .add_source(File::new("config.toml", FileFormat::Toml))
        .build()?;
    let source = configuration.get::<Source>("configuration.source");
    let api = Source::api;
    let local = Source::local;
    let NA = "N/A";
    if source == api {
        let client_config: ClientConfig = ClientConfig {
            source: source,
            file_path: NA.to_string(),
            api_key: configuration.get::<String>("configuration.api_key")?,
            symbols: configuration.get::<Vec<String>>("configuration.symbols")?,
            time_period: configuration.get::<i16>("configuration.time_period")?,
        };
    } else if source == local {
        let client_config: ClientConfig = ClientConfig {
            source: source,
            file_path: configuration.get::<String>("configuration.file_path")?,
            api_key: NA.to_string(),
            symbols: configuration.get::<Vec<String>>("configuration.symbols")?,
            time_period: configuration.get::<i16>("configuration.time_period")?,
        };
    }
    Ok(client_config)
}

fn make_api_call(client_config: ClientConfig) -> Result<SymbolData> {
    //need to make sure that this data doesnt keep calling if it reaches limit TODO
    let mut symbol_data = SymbolData;
    symbol_data.source = Source::api;
    symbol_data.time_period = client_config.time_period;
    for symbol in client_config.symbols {
        //how can I control time period though? I think this gives me max
        //This is non factor for optimization
        let mut easy = Easy::new();
        let mut response_data = Vec::new();
        let url = format!(
            "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY_ADJUSTED&symbol={}&apikey={}",
            symbol, client_config.api_key
        );
        easy.url(&url)?;
        {
            let mut transfer = easy.transfer();
            transfer.write_function(|new_data| {
                response_data.extend_from_slice(new_data);
                Ok(new_data.len())
            })?;
            transfer.perform()?;
        }
        //wtf am i doing here
        match response_data = get_stock_symbol_data(request_data) {
            Ok(response_data) => {
                symbols_data.data.extend(response_data);
                symbols_data.symbols.extend(symbol);
                continue
            }, 
            Err(..) => {
                println!("breaking loop max amount of symbol data sourced");
                break
            }
        }
    }
    Ok(symbols_data)
}
fn get_local_data(client_config: ClientConfig) -> Result<SymbolData> {
    let mut symbol_data = SymbolData;
    symbol_data.time_period = client_config.time_period;
    symbol_data.source = Source::local;
    let mut local_file = File::open(client_config.file_path)?;
    let json_data: Value = serde_json::from_read(file)?;
    symbol_data.data = json_data;
    symbols_data.symbols = client_config.symbols;
    Ok(symbols_data)
    //need to see if this form is the same as the api request... TODO
}

fn make_data_request(client_config: ClientConfig) -> Result<SymbolData> {
    let local = "local";
    let api = "api";
    if client_config.source == api {
        //call function for api data grab
        let api_symbol_data = make_api_call(client_config);
        api_symbol_data
    } else if client_config.source == local {
        //call function for local data grab
        let local_symbol_data = get_local_data(client_config);
        local_symbol_data
    } else {
        //error
        Err(anyhow::anyhow!("Inaccurate client config data source data"))
    }
}



//fn make_api_request(client_config: &ClientConfig, symbol: String) -> Result<Vec<u8>>{
//    let mut easy = Easy::new();
//    let mut request_data = Vec::new();
//    let url = format!(
//        "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY_ADJUSTED&symbol={}&apikey={}",
//        symbol, client_config.api_key
//    );
//    easy.url(&url)?;
//    {
//        let mut transfer = easy.transfer();
//        transfer.write_function(|new_data| {
//            request_data.extend_from_slice(new_data);
//            Ok(new_data.len())
//        })?;
//        transfer.perform()?;
//    }
////    let mut file = std::fs::File::create("testing_load.json")?;
////    file.write_all(&request_data)?;
//    Ok(request_data)
//}

fn get_stock_symbol_data(request_data: Vec<u8>) -> Result<serde_json::Value> {
    let response_body =
        str::from_utf8(&request_data).context("unable to convert request data from utf8")?;
    let symbols_data: Value = serde_json::from_str(&response_body)
        .context("unable to extract json from response body")?;
    Ok(symbols_data)
}


/*
 * Harvest_stock_metrics breakdown
 * TODO
 * (1) cleanse incoming data and handle a error well, fn process_symbol_data
 * (2) gather stock_data (is this useful?!!?!) Not sure on this one. maybe just use the
 * generate_stock_metrics function?
 */
//this makes one huge symbol data with its volumes.push() logic
//need to change to map of vectors symbol: volumes
//CUrrent TODO
fn generate_volume_from_timeseries(symbol_data: SymbolData) -> Result<HashMap<String, Vec<f64>>> {
    let mut symbols_volumes = HashMap::new();
    for symbol in symbol_data.data {
        let mut volumes = Vec::new>();
        if let Some(time_series) = symbols_data.data["Time Series (Daily)"].as_object() {
            //need to make it nested, quadractic I suppose
            for (_date, values) in time_series.into_iter().take(symbols_data.time_period.try_into()?) {
                let volume = values["6. volume"]
                    .as_str()
                    .ok_or_else(|| anyhow!("unable to get volume from timeseries"))?
                    .parse::<f64>()
                    .map_err(|_| anyhow!("unable to parse volume as f64"))?;
                volumes.push(volume);
            }
            //string : vec<f64>
            symbols_volumes.insert(symbol, volumes)
        } else {
            println!("time_series: {}", symbols_data.data);
            return Err(anyhow!("Time Series (Daily) data not found in symbols_data"));
        }
    }
    Ok(symbols_volumes)
}

fn generate_closings_from_timeseries(symbol_data: SymbolData) -> Result<Vec<f64>> {
    let mut closings = Vec::new();
    if let Some(time_series) = symbols_data.data["Time Series (Daily)"].as_object() {
        for (_date, values) in time_series.into_iter().take(symbols_data.time_period.try_into()?) {    
            let closing = values["4. close"]
                .as_str()
                .ok_or_else(|| anyhow!("unable to get closing price from timeseries"))?
                .parse::<f64>()
                .map_err(|_| anyhow!("unable to parse closing price as f64"))?;
            closings.push(closing);
        }
    } else {
        return Err(anyhow!("Time Series (Daily) data not found in symbols_data"));
    }
    Ok(closings)
}

//NEED to redsign this function TODO
fn generate_basic_metrics(
    closings: Vec<f64>,
    symbol_volume_set: HashMap<String, Vec<f64>>,
) -> Result<(Vec<BasicMetrics>, Vec<Vec<DailyStockData>>)> {
    //TODO INSPECTR RETURN HERE
    if(symbol_closings_set.len() !== symbol_volume_set.len()) {
        panic!
        //do something if they arent the same warning or something
    }
    let symbol_quantity = symbol_volume_set.len()
    let basic_metric_vector = Vec::new();
    for index in 0..symbol_quantity {
        let mut returns: Vec<f64> = Vec::new();
        let mut total_quantity: f64 = 0.0;
        let mut total_quantity_volume: f64 = 0.0;

        let mut daily_states: Vec<DailyStockData> = Vec::new();

        for i in 0..closings.len() - 1 {
            let change_in_value = closings[i + 1] - closings[i];
            let daily_return = change_in_value / closings[i];
            let daily_stock_data = DailyStockData {
                closing_price: closings[i],
                volume: volumes[i],
                change_in_value: change_in_value,
            };
            daily_states.push(daily_stock_data);
            total_quantity_volume += volumes[i];
            //CLOSINGS VECTOR IS MESSED UP
            total_quantity += closings[i];
            returns.push(daily_return);
        }
        let mean_return = returns.clone().mean();
        let variance = returns.variance();
        let standard_deviation = variance.sqrt();
        let mean_value = total_quantity / closings.len() as f64;
        let mean_volume = total_quantity_volume / volumes.len() as f64;
        let time_period = closings.len() as i16;

        Ok((BasicMetrics {
            mean_return: mean_return,
            mean_value: mean_value,
            mean_volume: mean_volume,
            variance: variance,
            standard_deviation: standard_deviation,
        }, daily_states))
}

fn apply_stock_metrics(
    basic_metrics: BasicMetrics,
    daily_states: Vec<DailyStockData>,
    symbol: String,
    time_period: i16
)   -> Result<StockData> {
    let stock = StockData {
        symbol: symbol.to_string(),
        basic_metrics: basic_metrics,
        time_period: time_period,
        daily_states: daily_states,
    };
    Ok(stock)
}

//#[allow(dead_code)]
fn compile_stock_data(
    client_requested_symbols: Vec<String>,
    api_key: &str,
    time_period: i16,
) -> Result<Vec<StockData>> {
    let mut compiled_data = Vec::new();
    //for symbol in client_requested_symbols {
        //TODO change this for new data model
    let config = grab_client_config()?;
    let symbol_data = make_data_request(config)?;
    //let test_request_data = response_for_load_testing()?;h /
    let volumes = generate_volume_from_timeseries(symbol_data)?;
    let closings = generate_closings_from_timeseries(symbol_data)?;
    let (basic_metrics, daily_states) = generate_basic_metrics(closings, volumes)?;
    //let stock = apply_stock_metrics(basic_metrics, daily_states, symbol, time_period)?;
    //compiled_data.push(stock);
    //}
    Ok(compiled_data)
}

fn handle_client_internal_interface() -> Result<Vec<StockData>> {
    let config = grab_client_config()?;
    let requested_symbols = config.symbols;
    let api_key = config.api_key;
    let time_period = config.time_period;
    Ok(compile_stock_data(requested_symbols, &api_key, time_period)?)
}

fn find_most_performant(stock: Vec<StockData>) -> Result<()> {
    let most_performant = stock
        .iter()
        .max_by(|a, b| {
            let adjusted_performance_a = a.basic_metrics.mean_return / a.basic_metrics.mean_value;
            let adjusted_performance_b = b.basic_metrics.mean_return / b.basic_metrics.mean_value;
            match (
                adjusted_performance_a.is_nan(),
                adjusted_performance_b.is_nan(),
            ) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (false, false) => adjusted_performance_a
                    .partial_cmp(&adjusted_performance_b)
                    .unwrap_or(std::cmp::Ordering::Equal),
            }
        })
        .context("unable to calculate most performant stock")?;

    println!("A higher performance score is better");
    println!("");
    println!(
        "Most performant stock in the last {} days...",
        most_performant.time_period
    );
    println!("Stock Symbol: {}", most_performant.symbol);
    println!(
        "Performance Score: {}",
        (most_performant.basic_metrics.mean_return / most_performant.basic_metrics.mean_value)*1000000.0
    );
    Ok(())
}

fn validate_api_key(api_key: &str) -> Result<bool> {
    let url = format!(
        "https://www.alphavantage.co/query?function=TIME_SERIES_INTRADAY&symbol=IBM&interval=5min&apikey={}",
        api_key
    );
    let mut easy = Easy::new();
    easy.url(&url)?;
    let mut response_body = Vec::new();
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|new_data| {
            response_body.extend_from_slice(new_data);
            Ok(new_data.len())
        })?;
        transfer.perform()?;
    }

    let response_str = std::str::from_utf8(&response_body)?;
    Ok(!response_str.contains("\"Note\""))
}

fn calculate_rsi(period_of_daily_stock_data: &Vec<DailyStockData>) -> Result<f64> {
    let mut losing_days = 0.0;
    let mut winning_days = 0.0;
    let mut avg_loss = 0.0;
    let mut avg_gain = 0.0;
     
    for daily_state in period_of_daily_stock_data {
        if daily_state.change_in_value < 0.0 {
            let negative_value_change = daily_state.change_in_value.abs();
            losing_days += 1.0;
            avg_loss += negative_value_change;
            let loss = negative_value_change;
        } else if daily_state.change_in_value > 0.0 {
            winning_days += 1.0;
            avg_gain += daily_state.change_in_value;
            let gain = daily_state.change_in_value;
        } else {
            continue;
        }
    }
    avg_loss = avg_loss / losing_days;
    avg_gain = avg_gain / winning_days;
    let mut rs: f64 = avg_gain/avg_loss;
    let mut rsi = 100.0 - (100.0/(1.0+rs));
    Ok(rsi)
}

// Unit tests
#[cfg(test)]
mod unit_tests {
    use super::*;

    use std::io::Write;
    use tempfile::NamedTempFile;
    #[test]
    fn test_grab_client_config() -> Result<()> {
        let mut temp_file = NamedTempFile::new().context("issue with tempfile")?;
        writeln!(temp_file, r#"
            [configuration]
            api_key = "demo"
            symbols = ["AAPL", "GOOG", "MSFT"]
            time_period = 10
        "#).context("unable to write to tempfile")?;

        let old_path = std::env::var("config.toml")?;
        std::env::set_var("config.toml", temp_file.path());

        let result = grab_client_config()?;

        std::env::set_var("config.toml", old_path);

        assert_eq!(result.api_key, "demo");
        assert_eq!(result.symbols, vec!["AAPL", "GOOG", "MSFT"]);
        assert_eq!(result.time_period, 10);
        Ok(())
    }

//    use mockito::{mock, Matcher};
//    #[test]
//    fn test_make_api_request() -> Result<()> {
//        let mut server = mockito::Server::new();
//        let host = server.host_with_port();
//        let url = server.url();
//
//        let _m = mock("GET", Matcher::Regex(r"^/query".to_string()))
//            .with_status(200)
//            .with_body("api response")
//            .create();
//        let client_config = ClientConfig {
//            api_key: "demo".to_string(),
//            symbols: vec!["test".to_string()],
//            time_period: 30,
//        };
//
//        let response = make_api_request(&client_config, "TEST".to_string()).unwrap();
//        assert_eq!(response, b"api response");
//        Ok(())
//    }
//
    #[test]
    fn test_get_stock_symbols_data() -> Result<()> {
        let expected_data = json!({
            "key": "value"
        });
        let json_string = expected_data.to_string();
        let json_bytes = json_string.into_bytes();

        let result = get_stock_symbol_data(json_bytes).context("get_stock_symbol_data test faulty")?;
        assert_eq!(result, expected_data);

        let invalid_utf8_bytes = vec![0, 159, 146, 150];  // Not valid UTF-8
        let result = get_stock_symbol_data(invalid_utf8_bytes);
        assert!(result.is_err());

        let invalid_json_bytes = b"{ invalid json".to_vec();  // Not valid JSON
        let result = get_stock_symbol_data(invalid_json_bytes);
        assert!(result.is_err());
        Ok(())
    }
}

fn main() -> Result<()> {
    //api validation test code
    //    let api_validation = validate_api_key("81I9AVPLTTFBVASS")?;
    //    if api_validation == true {
    //        println!("api key validated");
    //    value_changevalue_change  }
    let stock_data = handle_client_internal_interface()?;
    for stock in &stock_data {
        println!("Stock Symbol: {}", stock.symbol);
        println!(
            "Mean value return (last {} days): {}",
            stock.time_period, stock.basic_metrics.mean_return
        );
        println!(
            "Variance of value (last {} days): {}",
            stock.time_period, stock.basic_metrics.variance
        );
        println!(
            "Standard deviation of value (last {} days): {}",
            stock.time_period, stock.basic_metrics.standard_deviation
        );
        println!(
            "Mean value (last {} days): {}",
            stock.time_period, stock.basic_metrics.mean_value
        );
        let rsi = calculate_rsi(&stock.daily_states)?;
        println!(
            "RSI (last {} days): {}",
            stock.time_period,
            rsi
        );
        println!();
    }
    //give a specific *period of time*
    find_most_performant(stock_data)?;
    Ok(())
}
