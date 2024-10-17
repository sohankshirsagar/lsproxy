# lsproxy_sdk.CrateApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**definition**](CrateApi.md#definition) | **GET** /definition | Get the definition of a symbol at a specific position in a file
[**file_symbols**](CrateApi.md#file_symbols) | **GET** /file-symbols | Get symbols in a specific file
[**references**](CrateApi.md#references) | **GET** /references | Find all references to a symbol
[**workspace_files**](CrateApi.md#workspace_files) | **GET** /workspace-files | Get a list of all files in the workspace
[**workspace_symbols**](CrateApi.md#workspace_symbols) | **GET** /workspace-symbols | Search for symbols across the entire workspace


# **definition**
> DefinitionResponse definition(position, include_raw_response=include_raw_response)

Get the definition of a symbol at a specific position in a file

Get the definition of a symbol at a specific position in a file  Returns the location of the definition for the symbol at the given position.

### Example


```python
import lsproxy_sdk
from lsproxy_sdk.models.definition_response import DefinitionResponse
from lsproxy_sdk.models.file_position import FilePosition
from lsproxy_sdk.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = lsproxy_sdk.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with lsproxy_sdk.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = lsproxy_sdk.CrateApi(api_client)
    position = lsproxy_sdk.FilePosition() # FilePosition | The position within the file to get the definition for. This should point to the identifier of the symbol you want to get the definition for.  e.g. for getting the definition of `User` on line 10 of `src/main.py` with the code: ``` 0: class User: 1:     def __init__(self, name, age): 2:         self.name = name 3:         self.age = age 4: 5: user = User(\"John\", 30) __________^^^ ``` The (line, char) should be anywhere in (5, 7)-(5, 11).
    include_raw_response = True # bool | Whether to include the raw response from the langserver in the response. Defaults to false. (optional)

    try:
        # Get the definition of a symbol at a specific position in a file
        api_response = api_instance.definition(position, include_raw_response=include_raw_response)
        print("The response of CrateApi->definition:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->definition: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **position** | [**FilePosition**](.md)| The position within the file to get the definition for. This should point to the identifier of the symbol you want to get the definition for.  e.g. for getting the definition of &#x60;User&#x60; on line 10 of &#x60;src/main.py&#x60; with the code: &#x60;&#x60;&#x60; 0: class User: 1:     def __init__(self, name, age): 2:         self.name &#x3D; name 3:         self.age &#x3D; age 4: 5: user &#x3D; User(\&quot;John\&quot;, 30) __________^^^ &#x60;&#x60;&#x60; The (line, char) should be anywhere in (5, 7)-(5, 11). | 
 **include_raw_response** | **bool**| Whether to include the raw response from the langserver in the response. Defaults to false. | [optional] 

### Return type

[**DefinitionResponse**](DefinitionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Definition retrieved successfully |  -  |
**400** | Bad request |  -  |
**500** | Internal server error |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **file_symbols**
> SymbolResponse file_symbols(file_path, include_raw_response=include_raw_response)

Get symbols in a specific file

Get symbols in a specific file  Returns a list of symbols (functions, classes, variables, etc.) defined in the specified file.

### Example


```python
import lsproxy_sdk
from lsproxy_sdk.models.symbol_response import SymbolResponse
from lsproxy_sdk.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = lsproxy_sdk.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with lsproxy_sdk.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = lsproxy_sdk.CrateApi(api_client)
    file_path = 'file_path_example' # str | The path to the file to get the symbols for, relative to the root of the workspace.
    include_raw_response = True # bool | Whether to include the raw response from the langserver in the response. Defaults to false. (optional)

    try:
        # Get symbols in a specific file
        api_response = api_instance.file_symbols(file_path, include_raw_response=include_raw_response)
        print("The response of CrateApi->file_symbols:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->file_symbols: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **file_path** | **str**| The path to the file to get the symbols for, relative to the root of the workspace. | 
 **include_raw_response** | **bool**| Whether to include the raw response from the langserver in the response. Defaults to false. | [optional] 

### Return type

[**SymbolResponse**](SymbolResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Symbols retrieved successfully |  -  |
**400** | Bad request |  -  |
**500** | Internal server error |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **references**
> ReferenceResponse references(symbol_identifier_position, include_declaration=include_declaration, include_raw_response=include_raw_response)

Find all references to a symbol

Find all references to a symbol  Returns a list of locations where the symbol at the given position is referenced.

### Example


```python
import lsproxy_sdk
from lsproxy_sdk.models.file_position import FilePosition
from lsproxy_sdk.models.reference_response import ReferenceResponse
from lsproxy_sdk.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = lsproxy_sdk.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with lsproxy_sdk.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = lsproxy_sdk.CrateApi(api_client)
    symbol_identifier_position = lsproxy_sdk.FilePosition() # FilePosition | The position within the file to get the references for. This should point to the identifier of the definition.  e.g. for getting the references of `User` on line 0 of `src/main.py` with the code: ``` 0: class User: _________^^^^ 1:     def __init__(self, name, age): 2:         self.name = name 3:         self.age = age 4: 5: user = User(\"John\", 30) ```
    include_declaration = True # bool | Whether to include the declaration (definition) of the symbol in the response. Defaults to false. (optional)
    include_raw_response = True # bool | Whether to include the raw response from the langserver in the response. Defaults to false. (optional)

    try:
        # Find all references to a symbol
        api_response = api_instance.references(symbol_identifier_position, include_declaration=include_declaration, include_raw_response=include_raw_response)
        print("The response of CrateApi->references:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->references: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **symbol_identifier_position** | [**FilePosition**](.md)| The position within the file to get the references for. This should point to the identifier of the definition.  e.g. for getting the references of &#x60;User&#x60; on line 0 of &#x60;src/main.py&#x60; with the code: &#x60;&#x60;&#x60; 0: class User: _________^^^^ 1:     def __init__(self, name, age): 2:         self.name &#x3D; name 3:         self.age &#x3D; age 4: 5: user &#x3D; User(\&quot;John\&quot;, 30) &#x60;&#x60;&#x60; | 
 **include_declaration** | **bool**| Whether to include the declaration (definition) of the symbol in the response. Defaults to false. | [optional] 
 **include_raw_response** | **bool**| Whether to include the raw response from the langserver in the response. Defaults to false. | [optional] 

### Return type

[**ReferenceResponse**](ReferenceResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | References retrieved successfully |  -  |
**400** | Bad request |  -  |
**500** | Internal server error |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **workspace_files**
> List[str] workspace_files()

Get a list of all files in the workspace

Get a list of all files in the workspace  Returns an array of file paths for all files in the current workspace.  This is a convenience endpoint that does not use the underlying Language Servers directly, but it does apply the same filtering.

### Example


```python
import lsproxy_sdk
from lsproxy_sdk.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = lsproxy_sdk.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with lsproxy_sdk.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = lsproxy_sdk.CrateApi(api_client)

    try:
        # Get a list of all files in the workspace
        api_response = api_instance.workspace_files()
        print("The response of CrateApi->workspace_files:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->workspace_files: %s\n" % e)
```



### Parameters

This endpoint does not need any parameter.

### Return type

**List[str]**

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Workspace files retrieved successfully |  -  |
**400** | Bad request |  -  |
**500** | Internal server error |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **workspace_symbols**
> SymbolResponse workspace_symbols(query, include_raw_response=include_raw_response)

Search for symbols across the entire workspace

Search for symbols across the entire workspace  Returns a list of symbols matching the given query string from all files in the workspace.

### Example


```python
import lsproxy_sdk
from lsproxy_sdk.models.symbol_response import SymbolResponse
from lsproxy_sdk.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = lsproxy_sdk.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with lsproxy_sdk.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = lsproxy_sdk.CrateApi(api_client)
    query = 'query_example' # str | The query to search for.
    include_raw_response = True # bool | Whether to include the raw response from the langserver in the response. Defaults to false. (optional)

    try:
        # Search for symbols across the entire workspace
        api_response = api_instance.workspace_symbols(query, include_raw_response=include_raw_response)
        print("The response of CrateApi->workspace_symbols:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->workspace_symbols: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **query** | **str**| The query to search for. | 
 **include_raw_response** | **bool**| Whether to include the raw response from the langserver in the response. Defaults to false. | [optional] 

### Return type

[**SymbolResponse**](SymbolResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Workspace symbols retrieved successfully |  -  |
**400** | Bad request |  -  |
**500** | Internal server error |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

