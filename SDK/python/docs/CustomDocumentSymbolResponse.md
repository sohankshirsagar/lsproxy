# CustomDocumentSymbolResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**document_symbols** | [**List[SimplifiedDocumentSymbol]**](SimplifiedDocumentSymbol.md) |  | 
**raw_response** | **object** |  | 

## Example

```python
from openapi_client.models.custom_document_symbol_response import CustomDocumentSymbolResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CustomDocumentSymbolResponse from a JSON string
custom_document_symbol_response_instance = CustomDocumentSymbolResponse.from_json(json)
# print the JSON string representation of the object
print(CustomDocumentSymbolResponse.to_json())

# convert the object into a dict
custom_document_symbol_response_dict = custom_document_symbol_response_instance.to_dict()
# create an instance of CustomDocumentSymbolResponse from a dict
custom_document_symbol_response_from_dict = CustomDocumentSymbolResponse.from_dict(custom_document_symbol_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


