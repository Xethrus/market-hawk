use curl::easy::Easy;
use serde_json::Value;
use std::str;
use statrs::statistics::Statistics;
use std::error::Error;

struct StockData {
    mean_return: f64,
    variance: f64,
    standard_deviation: f64,
}

fn fetch_stock_symbols_data(symbol: &str, api_key: &str) -> Result<(), Box<dyn std::error::Error>>{
    let mut easy = Easy::new();
    let mut request_data = Vec::new();
    let url = format!("https://www.alphavantage.co/query?function=TIME_SERIES_DAILY&symbol={}&apikey={}", symbol, api_key);
    easy.url(&url)?;
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|new_data| {
            request_data.extend_from_slice(new_data);
            Ok(new_data.len())
        })?;
        transfer.perform()?;
    }
    let response_body = str::from_utf8(&request_data).unwrap();
    let data: Value = serde_json::from_str(&response_body).unwrap();
    let timeseries = data["Time Series (Daily)"].as_object().unwrap();

    let mut closing_prices_all = Vec::new();
    let mut closing_prices_30 = Vec::new();
    
    for (_date, values) in timeseries {
        let close = values["4. close"].as_str().unwrap().parse::<f64>().unwrap();
        closing_prices_all.push(close);
    }

    closing_prices_30 = closing_prices_all.clone();
    closing_prices_30.reverse(); // reverse the vector to start from the most recent date
    closing_prices_30.truncate(30); // limit the entries to the last 30 days

    let stock_data_all = calculate_close_differences(closing_prices_all)?;
    let stock_data_30 = calculate_close_differences(closing_prices_30)?;
    
    println!("{} mean value (all): {}",symbol, stock_data_all.mean_return);
    println!("{} variance of value (all): {}",symbol, stock_data_all.variance);
    println!("{} standard deviation of value (all): {}",symbol, stock_data_all.standard_deviation);
    
    println!("{} mean value (last 30 days): {}",symbol, stock_data_30.mean_return);
    println!("{} variance of value (last 30 days): {}",symbol, stock_data_30.variance);
    println!("{} standard deviation of value (last 30 days): {}",symbol, stock_data_30.standard_deviation);

    Ok(())
}

fn calculate_close_differences(closing_prices: Vec<f64>) -> Result<StockData, Box<dyn Error>> {
    let mut returns: Vec<f64> = Vec::new();
    for i in 0..closing_prices.len() - 1 {
        let daily_return = (closing_prices[i + 1] - closing_prices[i]) / closing_prices[i];
        returns.push(daily_return);
    }
    let mean_return = returns.clone().mean();
    let variance = returns.variance();
    let standard_deviation = variance.sqrt();

    Ok(StockData{mean_return, variance, standard_deviation})
}

fn main() -> Result<(), Box<dyn Error>> {
    let symbol = "IBM";
    let api_key = "81I9AVPLTTFBVASS";
    fetch_stock_symbols_data(symbol, api_key)?;
    Ok(())
}

