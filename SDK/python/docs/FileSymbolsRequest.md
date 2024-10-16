# FileSymbolsRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**file_path** | **str** |  | 
**include_raw_response** | **bool** |  | [optional] 

## Example

```python
from lsproxy_sdk.models.file_symbols_request import FileSymbolsRequest

# TODO update the JSON string below
json = "{}"
# create an instance of FileSymbolsRequest from a JSON string
file_symbols_request_instance = FileSymbolsRequest.from_json(json)
# print the JSON string representation of the object
print(FileSymbolsRequest.to_json())

# convert the object into a dict
file_symbols_request_dict = file_symbols_request_instance.to_dict()
# create an instance of FileSymbolsRequest from a dict
file_symbols_request_from_dict = FileSymbolsRequest.from_dict(file_symbols_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


