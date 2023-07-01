* * * * * *
**structs**
* * * * * *
***StockData***
    This struct functions as a container for the calculated data surrounding a stock symbol
***DailyStockData***
    This struct functions as a container for the daily information surrounding a stock
***BasicMetrics***
    This struct functions as a container for the calculated metrics surrounding a stock
***SymbolData***
    This struct functions as a container for the unparsed data as a whole, with the goal of providing a greater data flow and encapsulation- for the ability to normalize data from local or api
***ClientConfig***
    This struct functions as a container for the parsed information from the clients config, information used to produce output. (used by client to define the data they want from the possibly generable data)


* * * * * * 
**enums**
* * * * * *
***Source***
    This enum works to provide binary options for defining the source of the data (whether api or local)


* * * * * * 
**functions**
* * * * * *
***grab_client_config() -> Result<ClientConfig, ConfigError>***
    This function works to grab the necessary information from the clients config file to define the basic form of the data and how it should be retrieved.

***make_api_call(client_config: ClientConfig) -> Result<SymbolData>***
    This function makes an api call for every symbol defined in the clients config, then adding this information to the SymbolData object. 
        It stops short if the expected data is not found in the api call forloop, as such it recoords the symbols from which the data was successfully extracted.
        It extends its vector of data within SymbolData, but maybe it would be better to store it in a Map with a Symbol - Vec<serde_json::Value> structure.
        It returns the SymboData object


