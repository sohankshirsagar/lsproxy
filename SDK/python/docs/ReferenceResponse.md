# ReferenceResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**raw_response** | **object** |  | 
**references** | [**List[FilePosition]**](FilePosition.md) |  | 

## Example

```python
from lsproxy_sdk.models.reference_response import ReferenceResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ReferenceResponse from a JSON string
reference_response_instance = ReferenceResponse.from_json(json)
# print the JSON string representation of the object
print(ReferenceResponse.to_json())

# convert the object into a dict
reference_response_dict = reference_response_instance.to_dict()
# create an instance of ReferenceResponse from a dict
reference_response_from_dict = ReferenceResponse.from_dict(reference_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


