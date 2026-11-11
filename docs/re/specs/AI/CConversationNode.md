# CConversationNode (AI) - RE-SPEC-FINAL group A

## 0x43A710 CConversationNode::ClearRecursively  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (CConversationNode* this)`, plain `ret`. Recursive; callers: itself (2 sites), CConversations::RemoveConversationForPed 0x43A960.
- Behaviour: if `this->m_NodeYes (+8, i16) >= 0` recurse on `CConversations::m_Nodes[m_NodeYes]` (0x969570 + 0x18*i); then if `m_NodeNo (+0xA, i16)
  >= 0` recurse on `m_Nodes[m_NodeNo]` (both fields re-read after the first recursion). Then stores in this order: `m_Name[0] (+0) = 0`,
  `m_NodeNo = -1`, `m_NodeYes = -1`, `+0x10 = 0` (m_SpeechY), `+0x14 = 0` (m_SpeechN), `+0xC = 0` (m_Speech).
- gta-reversed: ConversationNode.cpp:20 - same logic (gr's Clear() stores in a different order; no behavioural difference). No cycle guard
  (a node graph with a cycle recurses until stack overflow, same as gr). Open questions: none.
