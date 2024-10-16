# SimpleSymbolResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**raw_response** | **object** |  | 
**symbols** | [**List[SimpleSymbol]**](SimpleSymbol.md) |  | 

## Example

```python
from lsproxy_sdk.models.simple_symbol_response import SimpleSymbolResponse

# TODO update the JSON string below
json = "{}"
# create an instance of SimpleSymbolResponse from a JSON string
simple_symbol_response_instance = SimpleSymbolResponse.from_json(json)
# print the JSON string representation of the object
print(SimpleSymbolResponse.to_json())

# convert the object into a dict
simple_symbol_response_dict = simple_symbol_response_instance.to_dict()
# create an instance of SimpleSymbolResponse from a dict
simple_symbol_response_from_dict = SimpleSymbolResponse.from_dict(simple_symbol_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


