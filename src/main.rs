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
}

#[derive(Debug, Deserialize)]
struct ClientConfig {
    api_key: String,
    symbols: Vec<String>,
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
    fetch_stock_symbols_data(&symbols_str, &api_key)
}

fn fetch_stock_symbols_data(
    symbols: &[&str],
    api_key: &str,
) -> Result<(Vec<StockData>, Vec<StockData>)> {
    let mut stock_data_all_group = Vec::new();
    let mut stock_data_30_group = Vec::new();
    for symbol in symbols {
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
        let mut closing_prices_all = Vec::new();
        let timeseries = match data["Time Series (Daily)"].as_object() {
            Some(timeseries) => {
                for (_date, values) in timeseries {
                    let close = values["4. close"].as_str().unwrap().parse::<f64>().context("unable to grab close value from timeseries")?;
                    closing_prices_all.push(close);
                }
            },
            None => {
                println!("{:#?}", data); // TODO Top tier debug print
            }
        };



        let mut closing_prices_30 = closing_prices_all.clone();
        closing_prices_30.reverse(); // reverse the vector to start from the most recent date
        closing_prices_30.truncate(30); // limit the entries to the last 30 days

        let stock_data_all = calculate_close_differences(closing_prices_all, symbol)?;
        let stock_data_30 = calculate_close_differences(closing_prices_30, symbol)?;

        let stock_all = StockData {
            symbol: symbol.to_string(),
            mean_return: stock_data_all.mean_return,
            variance: stock_data_all.variance,
            standard_deviation: stock_data_all.standard_deviation,
            mean_value: stock_data_all.mean_value,
        };
        stock_data_all_group.push(stock_all);

        let stock_30 = StockData {
            symbol: symbol.to_string(),
            mean_return: stock_data_30.mean_return,
            variance: stock_data_30.variance,
            standard_deviation: stock_data_30.standard_deviation,
            mean_value: stock_data_30.mean_value,
        };
        stock_data_30_group.push(stock_30);
    }
    Ok((stock_data_all_group, stock_data_30_group))
}

fn calculate_close_differences(
    closing_prices: Vec<f64>,
    symbol: &str,
) -> Result<StockData> {
    let mut returns: Vec<f64> = Vec::new();
    let mut total_quantity: f64 = 0.0;
    for i in 0..closing_prices.len() - 1 {
        let daily_return = (closing_prices[i + 1] - closing_prices[i]) / closing_prices[i];
        total_quantity += closing_prices[i];
        returns.push(daily_return);
    }
    let mean_return = returns.clone().mean();
    let variance = returns.variance();
    let standard_deviation = variance.sqrt();
    //double checking TODO LOOKS GOOD
    //    println!("Number of closing prices: {}", closing_prices.len());
    let mean_value = total_quantity / closing_prices.len() as f64;

    Ok(StockData {
        symbol: symbol.to_string(),
        mean_return,
        variance,
        standard_deviation,
        mean_value,
    })
}
//TODO brain dead to this logic atm
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
