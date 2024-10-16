# SimpleSymbol


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**kind** | **str** |  | 
**location** | [**SimpleLocation**](SimpleLocation.md) |  | 
**name** | **str** |  | 

## Example

```python
from lsproxy_sdk.models.simple_symbol import SimpleSymbol

# TODO update the JSON string below
json = "{}"
# create an instance of SimpleSymbol from a JSON string
simple_symbol_instance = SimpleSymbol.from_json(json)
# print the JSON string representation of the object
print(SimpleSymbol.to_json())

# convert the object into a dict
simple_symbol_dict = simple_symbol_instance.to_dict()
# create an instance of SimpleSymbol from a dict
simple_symbol_from_dict = SimpleSymbol.from_dict(simple_symbol_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


