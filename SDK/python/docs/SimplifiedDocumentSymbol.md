# SimplifiedDocumentSymbol


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**character** | **int** |  | 
**kind** | **str** |  | 
**line** | **int** |  | 
**name** | **str** |  | 

## Example

```python
from openapi_client.models.simplified_document_symbol import SimplifiedDocumentSymbol

# TODO update the JSON string below
json = "{}"
# create an instance of SimplifiedDocumentSymbol from a JSON string
simplified_document_symbol_instance = SimplifiedDocumentSymbol.from_json(json)
# print the JSON string representation of the object
print(SimplifiedDocumentSymbol.to_json())

# convert the object into a dict
simplified_document_symbol_dict = simplified_document_symbol_instance.to_dict()
# create an instance of SimplifiedDocumentSymbol from a dict
simplified_document_symbol_from_dict = SimplifiedDocumentSymbol.from_dict(simplified_document_symbol_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


