use anyhow::{Context, Result};
use curl::easy::Easy;
use serde::Deserialize;
use serde_json::Value;

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

impl StockData {
    fn update(&mut self, api_key: &str) -> Result<()> {
        Ok(*self = harvest_stock_metrics(
            self.time_period,
            get_stock_symbol_data(&self.symbol, api_key)?,
            self.symbol.as_str(),
        )?)
    }
}

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


fn make_api_request(client_config: ClientConfig) -> Result<Vec<u8>>{
    let mut easy = Easy::new();
    let mut request_data = Vec::new();
    let url = format!(
        "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY_ADJUSTED&symbol={}&apikey={}",
        client_config.symbol, client_config.api_key
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
    request_data
}

fn get_stock_symbols_data(request_data: Vec<u8>) -> Result<serde_json::Value> {
    let response_body =
        str::from_utf8(&request_data).context("unable to convert request data from utf8")?;
    let data: Value = serde_json::from_str(&response_body)
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

fn generate_volume_from_timeseries(symbols_data: serde_json::Value, client_config: ClientConfig) -> Result<Vec<f64>> {
    let mut volumes = Vec::new();

    symbol_data["Time Series (Daily)"].as_object() {
        Some(symbol_data["Time Series (Daily)"]) => {
            for(_date, values) in symbols_data["Time Series (Daily)"].into_iter().take(client_config.time_period.try_into()?) {
                let volume = values["6. volume"]
                    .as_str()?
                    .parse::<f64>()
                    .context("unable to get volume from timeseries")?;
                volumes.push(volume);
            }
        }
    }
    volumes
}

fn generate_closing_from_timeseries(symbols_data: serde_json::Value, client_config: ClientConfig) -> Result<Vec<f64>> {
    let mut closings = Vec::new();
    symbol_data["Time Series (Daily)"].as_object() {
        Some(symbol_data["Time Series (Daily)"]) => {
            for(_date, values) in symbols_data["Time Series (Daily)"].into_iter().take(client_config.time_period.try_into()?) {
                let closing = values["4. close"]
                    .as_str()?
                    .parse::<f64>()
                    .context("unable to get volume from timeseries")?;
                closings.push(closing);
            }
        }
    }
    closings
}

fn generate_basic_metrics(
    closings: Vec<f64>,
    volumes: Vec<f64>,
) -> Result<BasicMetrics> {

    let mut returns: Vec<f64> = Vec::new();
    let mut total_quantity: f64 = 0.0;
    let mut total_quantity_volume: f64 = 0.0;

    let mut daily_states: Vec<DailyStockData> = Vec::new();

    for i in 0..closing_prices.len() - 1 {
        let change_in_value = closing_prices[i + 1] - closing_prices[i];
        let daily_return = change_in_value / closing_prices[i];
        let daily_stock_data = DailyStockData {
            closing_price: closing_prices[i],
            volume: volumes[i],
            change_in_value: change_in_value,
        };
        daily_states.push(daily_stock_data);
        total_quantity_volume += volumes[i];
        total_quantity += closing_prices[i];
        returns.push(daily_return);
    }
    let mean_return = returns.clone().mean();
    let variance = returns.variance();
    let standard_deviation = variance.sqrt();
    let mean_value = total_quantity / closing_prices.len() as f64;
    let mean_volume = total_quantity_volume / volumes.len() as f64;
    let time_period = closing_prices.len() as i16;

    Ok(BasicMetrics {
        mean_return: mean_return,
        mean_value: mean_value,
        mean_volume: mean_volume,
        variance: variance,
        standard_deviation: standard_deviation,
    },
    })
}

fn apply_stock_metrics(
    closing_prices: Vec<f64>,
    volumes: Vec<f64>
)   -> Result<StockData> {
    let stock_data = generate_stock_metrics(closing_prices.clone(), volumes.clone(), symbol)?;

    let stock_data = generate_stock_metrics(closing_prices.clone(), volumes.clone(), symbol)?;

    let stock = StockData {
        symbol: symbol.to_string(),
        basic_metrics: stock_data.basic_metrics,
        time_period: stock_data.time_period,
        daily_states: stock_data.daily_states,
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
        compiled_data.push(harvest_stock_metrics(
            time_period,
            get_stock_symbol_data(symbol, api_key)?,
            symbol,
        )?);
    }
    Ok(compiled_data)
}

fn handle_client_internal_interface() -> Result<Vec<StockData>> {
    let client_config = grab_client_config().context("unable to load client config")?;
    let api_key = &client_config.api_key;
    let symbols = &client_config.symbols;
    let time_period = &client_config.time_period;
    let symbols_str: Vec<&str> = client_config.symbols.iter().map(AsRef::as_ref).collect();
    Ok(compile_stock_data(symbols_str, &api_key, *time_period)?)
}
/* Generate stock metrics breakdown
 * (1) calculate metrics
 *  to me this function seems pretty simple, maybe make it more readable in general?
 *  to me it should just take a stockdata and modify it then return it though....t a
 */

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

fn main() -> Result<()> {
    //api validation test code
    //    let api_validation = validate_api_key("81I9AVPLTTFBVASS")?;
    //    if api_validation == true {
    //        println!("api key validated");
    //    }
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
