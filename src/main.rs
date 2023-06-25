use anyhow::{Result, Context};
use curl::easy::Easy;
use serde_json::Value;
use serde::Deserialize;


use statrs::statistics::Statistics;
use std::str;

use config::{Config, File, FileFormat};
use config::ConfigError;

struct StockData {
    symbol: String,
    mean_return: f64,
    variance: f64,
    standard_deviation: f64,
    mean_value: f64,
    time_period: f64,
    daily_states: Vec<DailyStockData>,
}

struct DailyStockData {
    price: f64,
    change_in_value: f64,
}

struct FlatMetrics {
    mean_return: f64,
    standard_deviation: f64,
    mean_value: f64,
    time_period: f64,
}

#[derive(Debug, Deserialize)]
struct ClientConfig {
    api_key: String,
    symbols: Vec<String>,
}

impl StockData {
    fn update(&mut self) {
        *self = self.harvest_stock_metrics(self.time_period, get_stock_symbol_data);
    }
}




fn get_stock_symbol_data(symbol: &str, api_key: &str) -> Result<String> {
    //TODO idk it depends on what serde json is returning for the return type here
    let mut easy = Easy::new();
    let mut request_data = Vec::new();
    let url = format!(
        "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY_ADJUSTED&symbol={}&apikey={}",
        symbol, api_key
    );
    easy.url(&url)?;
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|new_data| {
            request_data.extend_from_slice(new_data);
            Ok(new_data.len())
        })?;
        transfer.perform()?;
    }
    let response_body = str::from_utf8(&request_data).context("unable to convert request data from utf8")?;
    let data: Value = serde_json::from_str(&response_body).context("unable to extract json from response body")?;
    Ok(data)
}

fn harvest_stock_metrics(time_period: i16, symbol_data: String) -> Result<StockData> {
    let mut closing_prices = Vec::new();
    let timeseries = match symbol_data["Time Series (Daily)"].as_object() {
        Some(timeseries) => {
            //TODO fix error handling
            //TODO into iter might not work here but need to find a way to limit the for loop
            for (_date, values) in timeseries.into_iter().take(time_period) {
                    let close = values["4. close"].as_str().unwrap().parse::<f64>().context("unable to grab close value from timeseries")?;
                    closing_prices.push(close);
            }
        },
        None => {
                println!("{:#?}", data); // TODO Top tier debug print
        }
    };

    let stock_data = calculate_close_differences(closing_prices, symbol)?;

    let stock = StockData {
        symbol: symbol.to_string(),
        mean_return: stock_data.mean_return,
        variance: stock_data.variance,
        standard_deviation: stock_data.standard_deviation,
        mean_value: stock_data.mean_value,
        time_period: stock_data.time_period,
        daily_states: stock_data.daily_states,
    };
    Ok(stock)
}

#[allow(dead_code)]
fn compile_stock_data(client_requested_symbols: Vec<&str>, api_key: &str, time_period: i16) -> Result<Vec<StockData>>{
    //TODO presentable output of all data === MIGHT NEED & ref for symbols and api key
    let mut compiled_data = new::Vec(); 
    for symbol in symbols {
        compiled_data.push(harvest_stock_metrics(time_period, get_stock_symbol_data(symbol, api_key)));
    }
    Ok(compiled_data)
}


fn grab_client_config() -> Result<ClientConfig, ConfigError> {
    let configuration = Config::default();
    let configuration = Config::builder().add_source(File::new("config.toml", FileFormat::Toml)).build()?;
    let client_config: ClientConfig = ClientConfig {
        api_key: configuration.get::<String>("configuration.api_key")?,
        symbols: configuration.get::<Vec<String>>("configuration.symbols")?,
    };
    Ok(client_config)
}

fn handle_client_internal_interface() -> Result<(Vec<StockData>, Vec<StockData>)> {
    let client_config = grab_client_config().context("unable to load client config")?;
    let api_key = &client_config.api_key;
    let symbols = &client_config.symbols;
    let symbols_str: Vec<&str> = client_config.symbols.iter().map(AsRef::as_ref).collect();
    compile_stock_data(&symbols_str, &api_key)
}

fn generate_stock_metrics(
    closing_prices: Vec<f64>,
    symbol: &str,
) -> Result<StockData> {
    let mut returns: Vec<f64> = Vec::new();
    let mut total_quantity: f64 = 0.0;

    let mut daily_states: Vec<DailyStockData> = Vec::new();

    for i in 0..closing_prices.len() - 1 {
        let change_in_value = closing_prices[i+1] - closing_prices[i];
        let daily_return = change_in_value / closing_prices[i];
        let daily_stock_data = DailyStockData {
            price: closing_prices[i],
            change_in_value: change_in_value,
        };
        daily_states.push(daily_stock_data);

        total_quantity += closing_prices[i];
        returns.push(daily_return);
    }
    let mean_return = returns.clone().mean();
    let variance = returns.variance();
    let standard_deviation = variance.sqrt();
    //double checking TODO LOOKS GOOD
    //    println!("Number of closing prices: {}", closing_prices.len());
    let mean_value = total_quantity / closing_prices.len() as f64;
    let time_period = closing_prices.len() as f64;
    //TODO weird layered logic here StockData -> StockData
    Ok(StockData {
        symbol: symbol.to_string(),
        mean_return,
        variance,
        standard_deviation,
        mean_value,
        time_period,
        daily_states,
    })
}
//TODO brain dead to this logic atm
//

fn find_most_performant(stock_30: Vec<StockData>, stock_all: Vec<StockData>) -> Result<()> {
    let most_performant_30 = stock_30
        .iter()
        .max_by(|a, b| {
            let adjusted_performance_a = a.mean_return / a.mean_value;
            let adjusted_performance_b = b.mean_return / b.mean_value;
            adjusted_performance_a
                .partial_cmp(&adjusted_performance_b)
                .unwrap()
        })
        .context("unable to calculate most performant stock (30)")?;

    let most_performant_all = stock_all
        .iter()
        .max_by(|a, b| {
            let adjusted_performance_a = a.mean_return / a.mean_value;
            let adjusted_performance_b = b.mean_return / b.mean_value;
            match (adjusted_performance_a.is_nan(), adjusted_performance_b.is_nan()) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (false, false) => adjusted_performance_a.partial_cmp(&adjusted_performance_b).unwrap_or(std::cmp::Ordering::Equal),
            }
        })
        .context("unable to calculate most performant stock (all)")?;

    println!("A higher performance score is better");
    println!("");
    println!("Most performant stock in the last 30 days...");
    println!("Stock Symbol: {}", most_performant_30.symbol);
    println!(
        "Performance Score: {}",
        most_performant_30.mean_return / most_performant_30.mean_value
    );
    println!();
    println!("Most performant stock historically...");
    println!("Stock Symbol: {}", most_performant_all.symbol);
    println!(
        "Performance Score: {}",
        most_performant_all.mean_return / most_performant_all.mean_value
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

fn main() -> Result<()> {
    //api validation test code
//    let api_validation = validate_api_key("81I9AVPLTTFBVASS")?;
//    if api_validation == true {
//        println!("api key validated");
//    }
    let (stock_data_all, stock_data_30) = handle_client_internal_interface()?;
    for stock in &stock_data_30 {
        println!("Stock Symbol: {}", stock.symbol);
        println!("Mean value return (last 30 days): {}", stock.mean_return);
        println!("Variance of value (last 30 days): {}", stock.variance);
        println!(
            "Standard deviation of value (last 30 days): {}",
            stock.standard_deviation
        );
        println!("Mean value (last 30 days): {}", stock.mean_value);
        println!();
    }
    for stock in &stock_data_all {
        println!("Stock Symbol: {}", stock.symbol);
        println!("Mean value return (last 100 days): {}", stock.mean_return);
        println!("Variance of value (last 100 days): {}", stock.variance);
        println!(
            "Standard deviation of value (last 100 days): {}",
            stock.standard_deviation
        );
        println!("Mean value (last 100 days): {}", stock.mean_value);
        println!();
    }
    find_most_performant(stock_data_30, stock_data_all)?;
    Ok(())
}
