# ReferenceResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**raw_response** | **object** | The raw response from the langserver.  https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_references | [optional] 
**references** | [**List[FilePosition]**](FilePosition.md) | The references to the symbol. Points to the start position of the symbol&#39;s identifier.  e.g. for the references of &#x60;User&#x60; on line 0 character 6 of &#x60;src/main.py&#x60; with the code: &#x60;&#x60;&#x60; 0: class User: _________^ 1:     def __init__(self, name, age): 2:         self.name &#x3D; name 3:         self.age &#x3D; age 4: 5: user &#x3D; User(\&quot;John\&quot;, 30) _________^ 6: 7: print(user.name) &#x60;&#x60;&#x60; The references will be &#x60;[{\&quot;path\&quot;: \&quot;src/main.py\&quot;, \&quot;line\&quot;: 5, \&quot;character\&quot;: 7}]&#x60;.  | 

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


