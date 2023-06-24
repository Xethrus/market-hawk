use anyhow::{Result, Context};
use curl::easy::Easy;
use serde_json::Value;

use statrs::statistics::Statistics;
use std::error::Error;
use std::str;

use config::{Config, File, FileFormat};
use std::collections::HashMap;

struct StockData {
    symbol: String,
    mean_return: f64,
    variance: f64,
    standard_deviation: f64,
    mean_value: f64,
}

fn grab_client_config() -> Result<(Config, config::ConfigError)> {
    let mut configuration = Config::default();
    configuration.merge(File::new("config", FileFormat::Toml))?;
    Ok(configuration)
}

fn handle_client_internal_interface() -> Result<(Vec<StockData>, Vec<StockData>)> {
    let config = grab_client_config().context("unable to load client config")?;
    let api_key: &str = config.get("api_key").as_str().context("unable to get api key from config")?;
    let symbols: Vec<String> = config.get("symbols").context("unable to get stock symbols from config")?;
    Ok(fetch_stock_symbols_data(symbols, api_key))
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
            "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY&symbol={}&apikey={}",
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
        let timeseries = data["Time Series (Daily)"].as_object().context("unable to extract timeseries from json")?;

        let mut closing_prices_all = Vec::new();

        for (_date, values) in timeseries {
            let close = values["4. close"].as_str().unwrap().parse::<f64>().context("unable to grab close value from timeseries")?;
            closing_prices_all.push(close);
        }

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
fn find_most_performant(stock_30: Vec<StockData>, stock_all: Vec<StockData>) {
    let most_performant_30 = stock_30
        .iter()
        .max_by(|a, b| {
            let adjusted_performance_a = a.mean_return / a.mean_value;
            let adjusted_performance_b = b.mean_return / b.mean_value;
            adjusted_performance_a
                .partial_cmp(&adjusted_performance_b)
                .unwrap()
        })
        .unwrap();

    let most_performant_all = stock_all
        .iter()
        .max_by(|a, b| {
            let adjusted_performance_a = a.mean_return / a.mean_value;
            let adjusted_performance_b = b.mean_return / b.mean_value;
            adjusted_performance_a
                .partial_cmp(&adjusted_performance_b)
                .unwrap()
        })
        .unwrap();

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
}

fn main() -> Result<(), Box<dyn Error>> {
    let symbols = vec!["IBM", "AAPL", "GOOGL"];
    let api_key = "demo";
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
    find_most_performant(stock_data_30, stock_data_all);
    Ok(())
}
