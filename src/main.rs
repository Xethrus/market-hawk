use anyhow::{Context, Result};
use anyhow::anyhow;
use curl::easy::Easy;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

use statrs::statistics::Statistics;
use std::str;

use config::ConfigError;
use config::{Config, File, FileFormat};

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

#[derive(Debug, Deserialize)]
struct ClientConfig {
    api_key: String,
    symbols: Vec<String>,
    time_period: i16,
}

//impl StockData {
//    fn update(&mut self, api_key: &str) -> Result<()> {
//        Ok(*self = harvest_stock_metrics(
//            self.time_period,
//            get_stock_symbol_data(&self.symbol, api_key)?,
//            self.symbol.as_str(),
//        )?)
//    }
//}

fn grab_client_config() -> Result<ClientConfig, ConfigError> {
    let configuration = Config::default();
    let configuration = Config::builder()
        .add_source(File::new("config.toml", FileFormat::Toml))
        .build()?;
    let client_config: ClientConfig = ClientConfig {
        api_key: configuration.get::<String>("configuration.api_key")?,
        symbols: configuration.get::<Vec<String>>("configuration.symbols")?,
        time_period: configuration.get::<i16>("configuration.time_period")?,
    };
    Ok(client_config)
}


fn make_api_request(client_config: &ClientConfig, symbol: String) -> Result<Vec<u8>>{
    let mut easy = Easy::new();
    let mut request_data = Vec::new();
    let url = format!(
        "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY_ADJUSTED&symbol={}&apikey={}",
        symbol, client_config.api_key
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
    Ok(request_data)
}

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

fn generate_volume_from_timeseries(symbols_data: serde_json::Value, client_config: &ClientConfig) -> Result<Vec<f64>> {
    let mut volumes= Vec::new();
    if let Some(time_series) = symbols_data["Time Series (Daily)"].as_object() {
        for (_date, values) in time_series.into_iter().take(client_config.time_period.try_into()?) {
            let volume = values["6. volume"]
                .as_str()
                .ok_or_else(|| anyhow!("unable to get volume from timeseries"))?
                .parse::<f64>()
                .map_err(|_| anyhow!("unable to parse volume as f64"))?;
            volumes.push(volume);
        }
    } else {
        return Err(anyhow!("Time Series (Daily) data not found in symbols_data"));
    }
    Ok(volumes)
}

fn generate_closings_from_timeseries(symbols_data: serde_json::Value, client_config: ClientConfig) -> Result<Vec<f64>> {
    let mut closings = Vec::new();
    if let Some(time_series) = symbols_data["Time Series (Daily)"].as_object() {
        for (_date, values) in time_series.into_iter().take(client_config.time_period.try_into()?) {    
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

fn generate_basic_metrics(
    closings: Vec<f64>,
    volumes: Vec<f64>,
) -> Result<(BasicMetrics, Vec<DailyStockData>)> {

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
    for symbol in client_requested_symbols {
        let config = grab_client_config()?;
        let request_data = make_api_request(&config, symbol.clone())?;
        let stock_symbol_data = get_stock_symbol_data(request_data)?;
        let volumes = generate_volume_from_timeseries(stock_symbol_data.clone(), &config)?;
        let closings = generate_closings_from_timeseries(stock_symbol_data, config)?;
        let (basic_metrics, daily_states) = generate_basic_metrics(volumes, closings)?;
        let stock = apply_stock_metrics(basic_metrics, daily_states, symbol, time_period)?;
        compiled_data.push(stock);
    }
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
        most_performant.basic_metrics.mean_return / most_performant.basic_metrics.mean_value
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

fn calculate_rsi(daily_states: Vec<DailyStockData>, time_period: u16) -> Result<f64> {
    let mut losing_days = 0.0;
    let mut winning_days = 0.0;
    let mut avg_loss = 0.0;
    let mut avg_gain = 0.0;
    for state in daily_states {
        if state.change_in_value < 0.0 {
            losing_days += 1.0;
            avg_loss += state.change_in_value;
        } else if state.change_in_value > 0.0 {
            winning_days += 1.0;
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

    use mockito::{mock, Matcher};
    #[test]
    fn test_make_api_request() -> Result<()> {
        let mut server = mockito::Server::new();
        let host = server.host_with_port();
        let url = server.url();

        let _m = mock("GET", Matcher::Regex(r"^/query".to_string()))
            .with_status(200)
            .with_body("api response")
            .create();
        let client_config = ClientConfig {
            api_key: "demo".to_string(),
            symbols: vec!["test".to_string()],
            time_period: 30,
        };

        let response = make_api_request(&client_config, "TEST".to_string()).unwrap();
        assert_eq!(response, b"api response");
        Ok(())
    }

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
        println!();
    }
    find_most_performant(stock_data)?;
    Ok(())
}
