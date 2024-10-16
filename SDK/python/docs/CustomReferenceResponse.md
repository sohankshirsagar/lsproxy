# CustomReferenceResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**raw_response** | **object** |  | 
**references** | [**List[SimplifiedLocation]**](SimplifiedLocation.md) |  | 

## Example

```python
from openapi_client.models.custom_reference_response import CustomReferenceResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CustomReferenceResponse from a JSON string
custom_reference_response_instance = CustomReferenceResponse.from_json(json)
# print the JSON string representation of the object
print(CustomReferenceResponse.to_json())

# convert the object into a dict
custom_reference_response_dict = custom_reference_response_instance.to_dict()
# create an instance of CustomReferenceResponse from a dict
custom_reference_response_from_dict = CustomReferenceResponse.from_dict(custom_reference_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


