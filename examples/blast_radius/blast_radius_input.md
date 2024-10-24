# Diff of the change
```
diff --git a/clients/ts-sdk/openapi.json b/clients/ts-sdk/openapi.json
index 1e8ac6dd..d4146e31 100644
--- a/clients/ts-sdk/openapi.json
+++ b/clients/ts-sdk/openapi.json
@@ -7559,6 +7559,17 @@
           "weight": 0.5
         }
       },
+      "ContextOptions": {
+        "type": "object",
+        "description": "Context options to use for the completion. If not specified, all options will default to false.",
+        "properties": {
+          "include_links": {
+            "type": "boolean",
+            "description": "Include links in the context such that model can construct source URLs. If not specified, this defaults to false.",
+            "nullable": true
+          }
+        }
+      },
       "CountChunkQueryResponseBody": {
         "type": "object",
         "required": [
@@ -8007,6 +8018,14 @@
             "description": "If concat user messages query is set to true, all of the user messages in the topic will be concatenated together and used as the search query. If not specified, this defaults to false. Default is false.",
             "nullable": true
           },
+          "context_options": {
+            "allOf": [
+              {
+                "$ref": "#/components/schemas/ContextOptions"
+              }
+            ],
+            "nullable": true
+          },
           "filters": {
             "allOf": [
               {
@@ -8686,6 +8705,14 @@
             "description": "If concat user messages query is set to true, all of the user messages in the topic will be concatenated together and used as the search query. If not specified, this defaults to false. Default is false.",
             "nullable": true
           },
+          "context_options": {
+            "allOf": [
+              {
+                "$ref": "#/components/schemas/ContextOptions"
+              }
+            ],
+            "nullable": true
+          },
           "filters": {
             "allOf": [
               {
@@ -11838,6 +11865,14 @@
             "description": "If concat user messages query is set to true, all of the user messages in the topic will be concatenated together and used as the search query. If not specified, this defaults to false. Default is false.",
             "nullable": true
           },
+          "context_options": {
+            "allOf": [
+              {
+                "$ref": "#/components/schemas/ContextOptions"
+              }
+            ],
+            "nullable": true
+          },
           "filters": {
             "allOf": [
               {
diff --git a/clients/ts-sdk/src/types.gen.ts b/clients/ts-sdk/src/types.gen.ts
index 14964c20..797e39dd 100644
--- a/clients/ts-sdk/src/types.gen.ts
+++ b/clients/ts-sdk/src/types.gen.ts
@@ -395,6 +395,16 @@ export type ContentChunkMetadata = {
     weight: number;
 };
 
+/**
+ * Context options to use for the completion. If not specified, all options will default to false.
+ */
+export type ContextOptions = {
+    /**
+     * Include links in the context. If not specified, this defaults to false.
+     */
+    include_links?: (boolean) | null;
+};
+
 export type CountChunkQueryResponseBody = {
     count: number;
 };
@@ -532,6 +542,7 @@ export type CreateMessageReqPayload = {
      * If concat user messages query is set to true, all of the user messages in the topic will be concatenated together and used as the search query. If not specified, this defaults to false. Default is false.
      */
     concat_user_messages_query?: (boolean) | null;
+    context_options?: ((ContextOptions) | null);
     filters?: ((ChunkFilter) | null);
     highlight_options?: ((HighlightOptions) | null);
     llm_options?: ((LLMOptions) | null);
@@ -806,6 +817,7 @@ export type EditMessageReqPayload = {
      * If concat user messages query is set to true, all of the user messages in the topic will be concatenated together and used as the search query. If not specified, this defaults to false. Default is false.
      */
     concat_user_messages_query?: (boolean) | null;
+    context_options?: ((ContextOptions) | null);
     filters?: ((ChunkFilter) | null);
     highlight_options?: ((HighlightOptions) | null);
     llm_options?: ((LLMOptions) | null);
@@ -1808,6 +1820,7 @@ export type RegenerateMessageReqPayload = {
      * If concat user messages query is set to true, all of the user messages in the topic will be concatenated together and used as the search query. If not specified, this defaults to false. Default is false.
      */
     concat_user_messages_query?: (boolean) | null;
+    context_options?: ((ContextOptions) | null);
     filters?: ((ChunkFilter) | null);
     highlight_options?: ((HighlightOptions) | null);
     llm_options?: ((LLMOptions) | null);
diff --git a/frontends/chat/src/components/Layouts/MainLayout.tsx b/frontends/chat/src/components/Layouts/MainLayout.tsx
index 054cbb6b..b9e592f5 100644
--- a/frontends/chat/src/components/Layouts/MainLayout.tsx
+++ b/frontends/chat/src/components/Layouts/MainLayout.tsx
@@ -21,6 +21,7 @@ import { Topic } from "../../utils/apiTypes";
 import { HiOutlineAdjustmentsHorizontal } from "solid-icons/hi";
 import { FilterModal, Filters } from "../FilterModal";
 import { Popover, PopoverButton, PopoverPanel } from "terracotta";
+import { type ContextOptions } from "trieve-ts-sdk";
 
 export interface LayoutProps {
   setTopics: Setter<Topic[]>;
@@ -96,6 +97,8 @@ const MainLayout = (props: LayoutProps) => {
     createSignal<AbortController>(new AbortController());
   const [showFilterModal, setShowFilterModal] = createSignal<boolean>(false);
   const [searchType, setSearchType] = createSignal<string | null>("hybrid");
+  const [contextOptions, setContextOptions] =
+    createSignal<ContextOptions | null>(null);
 
   const handleReader = async (
     reader: ReadableStreamDefaultReader<Uint8Array>,
@@ -232,6 +235,7 @@ const MainLayout = (props: LayoutProps) => {
           },
           use_group_search: useGroupSearch(),
           search_type: searchType(),
+          context_options: contextOptions(),
         }),
         signal: completionAbortController().signal,
       });
@@ -461,6 +465,31 @@ const MainLayout = (props: LayoutProps) => {
                       }}
                     />
                   </div>
+                  <div class="flex w-full items-center gap-x-2">
+                    <label for="context_options.include_links">
+                      Include links in context:
+                    </label>
+                    <input
+                      type="checkbox"
+                      id="context_options.include_links"
+                      class="h-4 w-4 rounded-md border border-neutral-300 bg-neutral-100 p-1 dark:border-neutral-900 dark:bg-neutral-800"
+                      checked={contextOptions()?.include_links ?? false}
+                      onChange={(e) => {
+                        setContextOptions((prev) => {
+                          if (!prev) {
+                            return {
+                              include_links: e.target.checked,
+                            };
+                          } else {
+                            return {
+                              ...prev,
+                              include_links: e.target.checked,
+                            } as ContextOptions;
+                          }
+                        });
+                      }}
+                    />
+                  </div>
                   <div class="flex w-full items-center gap-x-2">
                     <label for="concat_user_messages">Use Images</label>
                     <input
diff --git a/server/src/data/models.rs b/server/src/data/models.rs
index 60649673..c2012cda 100644
--- a/server/src/data/models.rs
+++ b/server/src/data/models.rs
@@ -5905,6 +5905,21 @@ pub struct ImageConfig {
     pub images_per_chunk: Option<usize>,
 }
 
+#[derive(Serialize, Deserialize, Debug, ToSchema, Clone)]
+/// Context options to use for the completion. If not specified, all options will default to false.
+pub struct ContextOptions {
+    /// Include links in the context. If not specified, this defaults to false.
+    pub include_links: Option<bool>,
+}
+
+impl Default for ContextOptions {
+    fn default() -> Self {
+        ContextOptions {
+            include_links: Some(false),
+        }
+    }
+}
+
 #[derive(Debug, Serialize, Deserialize, ToSchema, Clone, Default)]
 /// LLM options to use for the completion. If not specified, this defaults to the dataset's LLM options.
 pub struct LLMOptions {
@@ -5998,6 +6013,20 @@ fn extract_sort_highlight_options(
     (sort_options, highlight_options)
 }
 
+fn extract_context_options(other: &mut HashMap<String, Value>) -> Option<ContextOptions> {
+    let mut context_options = ContextOptions::default();
+
+    if let Some(value) = other.remove("include_links") {
+        context_options.include_links = serde_json::from_value(value).ok();
+    }
+
+    if context_options.include_links.is_none() {
+        None
+    } else {
+        Some(context_options)
+    }
+}
+
 fn extract_llm_options(other: &mut HashMap<String, Value>) -> Option<LLMOptions> {
     let mut llm_options = LLMOptions::default();
 
@@ -6290,6 +6319,7 @@ impl<'de> Deserialize<'de> for CreateMessageReqPayload {
             pub llm_options: Option<LLMOptions>,
             pub user_id: Option<String>,
             pub use_group_search: Option<bool>,
+            pub context_options: Option<ContextOptions>,
             #[serde(flatten)]
             other: std::collections::HashMap<String, serde_json::Value>,
         }
@@ -6302,8 +6332,10 @@ impl<'de> Deserialize<'de> for CreateMessageReqPayload {
             (None, None)
         };
         let llm_options = extract_llm_options(&mut helper.other);
+        let context_options = extract_context_options(&mut helper.other);
         let highlight_options = helper.highlight_options.or(extracted_highlight_options);
         let llm_options = helper.llm_options.or(llm_options);
+        let context_options = helper.context_options.or(context_options);
 
         Ok(CreateMessageReqPayload {
             new_message_content: helper.new_message_content,
@@ -6318,6 +6350,7 @@ impl<'de> Deserialize<'de> for CreateMessageReqPayload {
             score_threshold: helper.score_threshold,
             llm_options,
             user_id: helper.user_id,
+            context_options,
         })
     }
 }
@@ -6340,6 +6373,7 @@ impl<'de> Deserialize<'de> for RegenerateMessageReqPayload {
             pub llm_options: Option<LLMOptions>,
             pub user_id: Option<String>,
             pub use_group_search: Option<bool>,
+            pub context_options: Option<ContextOptions>,
             #[serde(flatten)]
             other: std::collections::HashMap<String, serde_json::Value>,
         }
@@ -6352,8 +6386,10 @@ impl<'de> Deserialize<'de> for RegenerateMessageReqPayload {
             (None, None)
         };
         let llm_options = extract_llm_options(&mut helper.other);
+        let context_options = extract_context_options(&mut helper.other);
         let highlight_options = helper.highlight_options.or(extracted_highlight_options);
         let llm_options = helper.llm_options.or(llm_options);
+        let context_options = helper.context_options.or(context_options);
 
         Ok(RegenerateMessageReqPayload {
             topic_id: helper.topic_id,
@@ -6367,6 +6403,7 @@ impl<'de> Deserialize<'de> for RegenerateMessageReqPayload {
             score_threshold: helper.score_threshold,
             llm_options,
             user_id: helper.user_id,
+            context_options,
         })
     }
 }
@@ -6391,6 +6428,7 @@ impl<'de> Deserialize<'de> for EditMessageReqPayload {
             pub score_threshold: Option<f32>,
             pub llm_options: Option<LLMOptions>,
             pub user_id: Option<String>,
+            pub context_options: Option<ContextOptions>,
             #[serde(flatten)]
             other: std::collections::HashMap<String, serde_json::Value>,
         }
@@ -6403,8 +6441,10 @@ impl<'de> Deserialize<'de> for EditMessageReqPayload {
             (None, None)
         };
         let llm_options = extract_llm_options(&mut helper.other);
+        let context_options = extract_context_options(&mut helper.other);
         let highlight_options = helper.highlight_options.or(extracted_highlight_options);
         let llm_options = helper.llm_options.or(llm_options);
+        let context_options = helper.context_options.or(context_options);
 
         Ok(EditMessageReqPayload {
             topic_id: helper.topic_id,
@@ -6420,6 +6460,7 @@ impl<'de> Deserialize<'de> for EditMessageReqPayload {
             score_threshold: helper.score_threshold,
             user_id: helper.user_id,
             llm_options,
+            context_options,
         })
     }
 }
diff --git a/server/src/handlers/chunk_handler.rs b/server/src/handlers/chunk_handler.rs
index 99a65969..4987b3a2 100644
--- a/server/src/handlers/chunk_handler.rs
+++ b/server/src/handlers/chunk_handler.rs
@@ -1,12 +1,12 @@
 use super::auth_handler::{AdminOnly, LoggedUser};
 use crate::data::models::{
     escape_quotes, ChatMessageProxy, ChunkMetadata, ChunkMetadataStringTagSet,
-    ChunkMetadataWithScore, ConditionType, CountSearchMethod, DatasetAndOrgWithSubAndPlan,
-    DatasetConfiguration, GeoInfo, HighlightOptions, ImageConfig, IngestSpecificChunkMetadata,
-    Pool, QueryTypes, RagQueryEventClickhouse, RecommendType, RecommendationEventClickhouse,
-    RecommendationStrategy, RedisPool, ScoreChunk, ScoreChunkDTO, SearchMethod,
-    SearchQueryEventClickhouse, SlimChunkMetadataWithScore, SortByField, SortOptions, TypoOptions,
-    UnifiedId, UpdateSpecificChunkMetadata,
+    ChunkMetadataWithScore, ConditionType, ContextOptions, CountSearchMethod,
+    DatasetAndOrgWithSubAndPlan, DatasetConfiguration, GeoInfo, HighlightOptions, ImageConfig,
+    IngestSpecificChunkMetadata, Pool, QueryTypes, RagQueryEventClickhouse, RecommendType,
+    RecommendationEventClickhouse, RecommendationStrategy, RedisPool, ScoreChunk, ScoreChunkDTO,
+    SearchMethod, SearchQueryEventClickhouse, SlimChunkMetadataWithScore, SortByField, SortOptions,
+    TypoOptions, UnifiedId, UpdateSpecificChunkMetadata,
 };
 use crate::errors::ServiceError;
 use crate::get_env;
@@ -2347,6 +2347,8 @@ pub struct GenerateOffChunksReqPayload {
     pub user_id: Option<String>,
     /// Configuration for sending images to the llm
     pub image_config: Option<ImageConfig>,
+    /// Context options to use for the completion. If not specified, all options will default to false.
+    pub context_options: Option<ContextOptions>,
 }
 
 /// RAG on Specified Chunks
@@ -2395,10 +2397,9 @@ pub async fn generate_off_chunks(
     };
 
     let chunk_ids = data.chunk_ids.clone();
-
     let prompt = data.prompt.clone();
-
     let stream_response = data.stream_response;
+    let context_options = data.context_options.clone();
 
     let mut chunks =
         get_metadata_from_ids_query(chunk_ids, dataset_org_plan_sub.dataset.id, pool).await?;
@@ -2471,8 +2472,9 @@ pub async fn generate_off_chunks(
             .cmp(&data.chunk_ids.iter().position(|&id| id == b.id).unwrap())
     });
 
-    chunks.iter().enumerate().for_each(|(idx, bookmark)| {
-        let content = convert_html_to_text(&(bookmark.chunk_html.clone().unwrap_or_default()));
+    chunks.iter().enumerate().for_each(|(idx, chunk_metadata)| {
+        let content =
+            convert_html_to_text(&(chunk_metadata.chunk_html.clone().unwrap_or_default()));
         let first_2000_words = content
             .split_whitespace()
             .take(2000)
@@ -2480,13 +2482,26 @@ pub async fn generate_off_chunks(
             .join(" ");
 
         messages.push(ChatMessage::User {
-            content: ChatMessageContent::Text(format!("Doc {}: {}", idx + 1, first_2000_words)),
+            content: ChatMessageContent::Text(format!(
+                "Doc {}{}: {}",
+                idx + 1,
+                if context_options
+                    .as_ref()
+                    .is_some_and(|x| x.include_links.unwrap_or(false))
+                    && chunk_metadata.link.is_some()
+                {
+                    format!(" ({})", chunk_metadata.link.clone().unwrap_or_default())
+                } else {
+                    "".to_string()
+                },
+                first_2000_words
+            )),
             name: None,
         });
 
         if let Some(image_config) = &data.image_config {
             if image_config.use_images.unwrap_or(false) {
-                if let Some(image_urls) = bookmark.image_urls.clone() {
+                if let Some(image_urls) = chunk_metadata.image_urls.clone() {
                     let urls = image_urls
                         .iter()
                         .filter_map(|image| image.clone())
diff --git a/server/src/handlers/message_handler.rs b/server/src/handlers/message_handler.rs
index 6cd2d954..0c9f01ee 100644
--- a/server/src/handlers/message_handler.rs
+++ b/server/src/handlers/message_handler.rs
@@ -4,8 +4,8 @@ use super::{
 };
 use crate::{
     data::models::{
-        self, ChunkMetadata, DatasetAndOrgWithSubAndPlan, DatasetConfiguration, HighlightOptions,
-        LLMOptions, Pool, RedisPool, SearchMethod, SuggestType,
+        self, ChunkMetadata, ContextOptions, DatasetAndOrgWithSubAndPlan, DatasetConfiguration,
+        HighlightOptions, LLMOptions, Pool, RedisPool, SearchMethod, SuggestType,
     },
     errors::ServiceError,
     get_env,
@@ -23,7 +23,6 @@ use crate::{
     },
 };
 use actix_web::{web, HttpResponse};
-
 use itertools::Itertools;
 use openai_dive::v1::{
     api::Client,
@@ -103,6 +102,8 @@ pub struct CreateMessageReqPayload {
     pub score_threshold: Option<f32>,
     /// LLM options to use for the completion. If not specified, this defaults to the dataset's LLM options.
     pub llm_options: Option<LLMOptions>,
+    /// Context options to use for the completion. If not specified, all options will default to false.
+    pub context_options: Option<ContextOptions>,
 }
 
 /// Create message
@@ -318,6 +319,8 @@ pub struct RegenerateMessageReqPayload {
     pub llm_options: Option<LLMOptions>,
     /// The user_id is the id of the user who is making the request. This is used to track user interactions with the RAG results.
     pub user_id: Option<String>,
+    /// Context options to use for the completion. If not specified, all options will default to false.
+    pub context_options: Option<ContextOptions>,
 }
 
 #[derive(Serialize, Debug, ToSchema)]
@@ -348,6 +351,8 @@ pub struct EditMessageReqPayload {
     pub llm_options: Option<LLMOptions>,
     /// The user_id is the id of the user who is making the request. This is used to track user interactions with the RAG results.
     pub user_id: Option<String>,
+    /// Context options to use for the completion. If not specified, all options will default to false.
+    pub context_options: Option<ContextOptions>,
 }
 
 impl From<EditMessageReqPayload> for CreateMessageReqPayload {
@@ -365,6 +370,7 @@ impl From<EditMessageReqPayload> for CreateMessageReqPayload {
             score_threshold: data.score_threshold,
             llm_options: data.llm_options,
             user_id: data.user_id,
+            context_options: data.context_options,
         }
     }
 }
@@ -384,6 +390,7 @@ impl From<RegenerateMessageReqPayload> for CreateMessageReqPayload {
             score_threshold: data.score_threshold,
             llm_options: data.llm_options,
             user_id: data.user_id,
+            context_options: data.context_options,
         }
     }
 }
diff --git a/server/src/lib.rs b/server/src/lib.rs
index 7521313b..a24c5ea8 100644
--- a/server/src/lib.rs
+++ b/server/src/lib.rs
@@ -439,6 +439,7 @@ impl Modify for SecurityAddon {
             data::models::UserOrganization,
             data::models::QdrantSortBy,
             data::models::SortOptions,
+            data::models::ContextOptions,
             data::models::LLMOptions,
             data::models::ImageConfig,
             data::models::HighlightOptions,
diff --git a/server/src/operators/message_operator.rs b/server/src/operators/message_operator.rs
index f6c6e509..7b4b8a4d 100644
--- a/server/src/operators/message_operator.rs
+++ b/server/src/operators/message_operator.rs
@@ -646,8 +646,18 @@ pub async fn stream_response(
         .enumerate()
         .map(|(idx, chunk)| {
             format!(
-                "Doc {}: {}",
+                "Doc {}{}: {}",
                 idx + 1,
+                if create_message_req_payload
+                    .clone()
+                    .context_options
+                    .is_some_and(|x| x.include_links.unwrap_or(false))
+                    && chunk.link.is_some()
+                {
+                    format!(" ({})", chunk.link.clone().unwrap_or_default())
+                } else {
+                    "".to_string()
+                },
                 convert_html_to_text(&(chunk.chunk_html.clone().unwrap_or_default()))
             )
         })
diff --git a/yarn.lock b/yarn.lock
index fdf44cd3..034eb9a9 100644
--- a/yarn.lock
+++ b/yarn.lock
@@ -3125,19 +3125,12 @@ eslint-utils@^3.0.0:
   version "3.0.0"
   resolved "https://registry.yarnpkg.com/eslint-utils/-/eslint-utils-3.0.0.tgz#8aebaface7345bb33559db0a1f13a1d2d48c3672"
   integrity sha512-uuQC43IGctw68pJA1RgbQS8/NP7rch6Cwd4j3ZBtgo4/8Flj4eGE7ZYSZRN3iq5pVUv6GPdW5Z1RFleo84uLDA==
-  dependencies:
-    eslint-visitor-keys "^2.0.0"
 
 eslint-visitor-keys@^1.1.0:
   version "1.3.0"
   resolved "https://registry.yarnpkg.com/eslint-visitor-keys/-/eslint-visitor-keys-1.3.0.tgz#30ebd1ef7c2fdff01c3a4f151044af25fab0523e"
   integrity sha512-6J72N8UNa462wa/KFODt/PJ3IU60SDpC3QXC1Hjc1BXXpfL2C9R5+AU7jhe0F6GREqVMh4Juu+NY7xn+6dipUQ==
 
-eslint-visitor-keys@^2.0.0:
-  version "2.1.0"
-  resolved "https://registry.yarnpkg.com/eslint-visitor-keys/-/eslint-visitor-keys-2.1.0.tgz#f65328259305927392c938ed44eb0a5c9b2bd303"
-  integrity sha512-0rSmRBzXgDzIsD6mGdJgevzgezI534Cer5L/vyMX0kHzT/jiB43jRhd9YUlMGYLQy2zprNmoT8qasCGtY+QaKw==
-
 eslint-visitor-keys@^3.3.0, eslint-visitor-keys@^3.4.1, eslint-visitor-keys@^3.4.3:
   version "3.4.3"
   resolved "https://registry.yarnpkg.com/eslint-visitor-keys/-/eslint-visitor-keys-3.4.3.tgz#0cd72fe8550e3c2eae156a96a4dddcd1c8ac5800"
@@ -6974,7 +6967,16 @@ stethoskop@1.0.0:
   dependencies:
     node-statsd "0.1.1"
 
-"string-width-cjs@npm:string-width@^4.2.0", string-width@^4.1.0, string-width@^4.2.0, string-width@^4.2.3:
+"string-width-cjs@npm:string-width@^4.2.0":
+  version "4.2.3"
+  resolved "https://registry.yarnpkg.com/string-width/-/string-width-4.2.3.tgz#269c7117d27b05ad2e536830a8ec895ef9c6d010"
+  integrity sha512-wKyQRQpjJ0sIp62ErSZdGsjMJWsap5oRNihHhu6G7JVO/9jIB6UyevL+tXuOqrng8j/cxKTWyWUwvSTriiZz/g==
+  dependencies:
+    emoji-regex "^8.0.0"
+    is-fullwidth-code-point "^3.0.0"
+    strip-ansi "^6.0.1"
+
+string-width@^4.1.0, string-width@^4.2.0, string-width@^4.2.3:
   version "4.2.3"
   resolved "https://registry.yarnpkg.com/string-width/-/string-width-4.2.3.tgz#269c7117d27b05ad2e536830a8ec895ef9c6d010"
   integrity sha512-wKyQRQpjJ0sIp62ErSZdGsjMJWsap5oRNihHhu6G7JVO/9jIB6UyevL+tXuOqrng8j/cxKTWyWUwvSTriiZz/g==
@@ -7085,7 +7087,14 @@ stringify-object@3.3.0:
     is-obj "^1.0.1"
     is-regexp "^1.0.0"
 
-"strip-ansi-cjs@npm:strip-ansi@^6.0.1", strip-ansi@^6.0.0, strip-ansi@^6.0.1:
+"strip-ansi-cjs@npm:strip-ansi@^6.0.1":
+  version "6.0.1"
+  resolved "https://registry.yarnpkg.com/strip-ansi/-/strip-ansi-6.0.1.tgz#9e26c63d30f53443e9489495b2105d37b67a85d9"
+  integrity sha512-Y38VPSHcqkFrCpFnQ9vuSXmquuv5oXOKpGeT6aGrr3o3Gc9AlVa6JBfUSOCnbxGGZF+/0ooI7KrPuUSztUdU5A==
+  dependencies:
+    ansi-regex "^5.0.1"
+
+strip-ansi@^6.0.0, strip-ansi@^6.0.1:
   version "6.0.1"
   resolved "https://registry.yarnpkg.com/strip-ansi/-/strip-ansi-6.0.1.tgz#9e26c63d30f53443e9489495b2105d37b67a85d9"
   integrity sha512-Y38VPSHcqkFrCpFnQ9vuSXmquuv5oXOKpGeT6aGrr3o3Gc9AlVa6JBfUSOCnbxGGZF+/0ooI7KrPuUSztUdU5A==
@@ -8017,7 +8026,16 @@ wordwrap@^1.0.0:
   resolved "https://registry.yarnpkg.com/wordwrap/-/wordwrap-1.0.0.tgz#27584810891456a4171c8d0226441ade90cbcaeb"
   integrity sha512-gvVzJFlPycKc5dZN4yPkP8w7Dc37BtP1yczEneOb4uq34pXZcvrtRTmWV8W+Ume+XCxKgbjM+nevkyFPMybd4Q==
 
-"wrap-ansi-cjs@npm:wrap-ansi@^7.0.0", wrap-ansi@^7.0.0:
+"wrap-ansi-cjs@npm:wrap-ansi@^7.0.0":
+  version "7.0.0"
+  resolved "https://registry.yarnpkg.com/wrap-ansi/-/wrap-ansi-7.0.0.tgz#67e145cff510a6a6984bdf1152911d69d2eb9e43"
+  integrity sha512-YVGIj2kamLSTxw6NsZjoBxfSwsn0ycdesmc4p+Q21c5zPuZ1pl+NfxVdxPtdHvmNVOQ6XSYG4AUtyt/Fi7D16Q==
+  dependencies:
+    ansi-styles "^4.0.0"
+    string-width "^4.1.0"
+    strip-ansi "^6.0.0"
+
+wrap-ansi@^7.0.0:
   version "7.0.0"
   resolved "https://registry.yarnpkg.com/wrap-ansi/-/wrap-ansi-7.0.0.tgz#67e145cff510a6a6984bdf1152911d69d2eb9e43"
   integrity sha512-YVGIj2kamLSTxw6NsZjoBxfSwsn0ycdesmc4p+Q21c5zPuZ1pl+NfxVdxPtdHvmNVOQ6XSYG4AUtyt/Fi7D16Q==
```
# Call hierarchy 
