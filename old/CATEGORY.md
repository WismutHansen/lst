# Category Support Implementation Plan for Desktop & Mobile Apps

## Overview
This document outlines the implementation plan for adding category support to the lst-desktop and lst-mobile Tauri applications. The category system uses markdown headlines (`## Category Name`) to organize list items, with support for both uncategorized items and categorized sections.

## Current Status
- ✅ **Core Library (lst-core)**: Category data structures and parsing implemented
- ✅ **CLI (lst-cli)**: Full category support with `##category` inline syntax and `lst cat` commands
- ❌ **Desktop App (lst-desktop)**: No category support
- ❌ **Mobile App (lst-mobile)**: No category support

## Architecture Overview

### Data Structure (Already Implemented)
```rust
pub struct List {
    pub metadata: ListMetadata,
    pub uncategorized_items: Vec<ListItem>,  // Items before first headline
    pub categories: Vec<Category>,           // Categorized items
}

pub struct Category {
    pub name: String,
    pub items: Vec<ListItem>,
}
```

### File Format (Already Implemented)
```markdown
---
title: groceries
---

- [ ] bread ^abc12
- [ ] eggs ^def34

## Dairy
- [ ] milk ^ghi56
- [ ] cheese ^jkl78

## Produce
- [ ] apples ^mno90
```

## Implementation Plan

### Phase 1: Backend Integration (High Priority)

#### 1.1 Update Tauri Commands
**Files to modify:**
- `apps/lst-desktop/src-tauri/src/lib.rs`
- `apps/lst-mobile/src-tauri/src/lib.rs`

**Tasks:**
1. **Update existing commands** to handle new List structure:
   ```rust
   // Current: list.items
   // New: list.uncategorized_items + list.categories
   ```

2. **Add new Tauri commands** for category management:
   ```rust
   #[tauri::command]
   #[specta::specta]
   async fn create_category(list_name: String, category_name: String) -> Result<(), String>
   
   #[tauri::command]
   #[specta::specta]
   async fn move_item_to_category(list_name: String, item_anchor: String, category_name: Option<String>) -> Result<(), String>
   
   #[tauri::command]
   #[specta::specta]
   async fn delete_category(list_name: String, category_name: String) -> Result<(), String>
   
   #[tauri::command]
   #[specta::specta]
   async fn get_categories(list_name: String) -> Result<Vec<String>, String>
   ```

3. **Update TypeScript bindings** generation to include new Category types

#### 1.2 Fix Existing Commands
**Commands that need updates:**
- `get_list_items()` - Should return items from all categories
- `add_item()` - Should support category parameter
- `toggle_item()` - Should work across all categories
- `delete_item()` - Should work across all categories

### Phase 2: Frontend UI Components (High Priority)

#### 2.1 Update List Display Component
**Files to modify:**
- `apps/lst-desktop/src/components/ListDisplay.tsx`
- `apps/lst-mobile/src/components/ListDisplay.tsx`

**Tasks:**
1. **Modify list rendering** to show categories:
   ```tsx
   // Current: Single flat list
   // New: Uncategorized items + Category sections
   
   return (
     <div>
       {/* Uncategorized items */}
       {list.uncategorized_items.map(item => <ListItem key={item.anchor} item={item} />)}
       
       {/* Categorized items */}
       {list.categories.map(category => (
         <CategorySection key={category.name} category={category} />
       ))}
     </div>
   );
   ```

2. **Create CategorySection component**:
   ```tsx
   interface CategorySectionProps {
     category: Category;
     onItemToggle: (anchor: string) => void;
     onItemDelete: (anchor: string) => void;
     onCategoryDelete: (categoryName: string) => void;
   }
   ```

3. **Add category headers** with:
   - Category name display
   - Item count badge
   - Collapse/expand functionality
   - Category management menu (rename, delete)

#### 2.2 Update Add Item Component
**Files to modify:**
- `apps/lst-desktop/src/components/AddItemForm.tsx`
- `apps/lst-mobile/src/components/AddItemForm.tsx`

**Tasks:**
1. **Add category selection** to add item form:
   ```tsx
   <select value={selectedCategory} onChange={setSelectedCategory}>
     <option value="">No Category</option>
     {categories.map(cat => <option key={cat} value={cat}>{cat}</option>)}
     <option value="__new__">+ Create New Category</option>
   </select>
   ```

2. **Support `##category` inline syntax**:
   ```tsx
   // Parse "##dairy milk" -> category: "dairy", text: "milk"
   const parseItemInput = (input: string) => {
     if (input.startsWith('##')) {
       const spaceIndex = input.indexOf(' ');
       if (spaceIndex > 2) {
         return {
           category: input.slice(2, spaceIndex),
           text: input.slice(spaceIndex + 1)
         };
       }
     }
     return { category: null, text: input };
   };
   ```

3. **Add quick category creation** modal/dialog

### Phase 3: Category Management UI (Medium Priority)

#### 3.1 Category Management Panel
**New files to create:**
- `apps/lst-desktop/src/components/CategoryManager.tsx`
- `apps/lst-mobile/src/components/CategoryManager.tsx`

**Features:**
1. **Category list** with item counts
2. **Drag & drop** item movement between categories
3. **Category CRUD operations**:
   - Create new category
   - Rename category
   - Delete category (with confirmation)
   - Merge categories

#### 3.2 Context Menus & Actions
**Tasks:**
1. **Item context menu** additions:
   - "Move to Category" submenu
   - "Remove from Category" option

2. **Category context menu**:
   - Rename category
   - Delete category
   - Move all items to another category

### Phase 4: Enhanced UX Features (Low Priority)

#### 4.1 Visual Enhancements
1. **Category color coding** (optional user setting)
2. **Category icons** (user-selectable)
3. **Collapsible categories** with state persistence
4. **Category statistics** (completion percentage, item counts)

#### 4.2 Search & Filtering
1. **Filter by category** in search
2. **Category-aware search** results
3. **"Show only category X"** toggle

#### 4.3 Keyboard Shortcuts
**Desktop-specific shortcuts:**
- `Ctrl+Shift+C` - Create new category
- `Ctrl+M` - Move selected item to category
- `Ctrl+1-9` - Quick switch to category N
- `Tab` - Cycle through categories

### Phase 5: Mobile-Specific Considerations

#### 5.1 Mobile UI Adaptations
1. **Swipe gestures** for category management:
   - Swipe right on item → Move to category menu
   - Swipe left on category header → Category options

2. **Touch-friendly category selection**:
   - Large touch targets
   - Bottom sheet for category selection
   - Haptic feedback for actions

3. **Responsive category display**:
   - Horizontal scrolling for many categories
   - Compact view for small screens

## Implementation Timeline

### Week 1: Backend Foundation
- [ ] Update Tauri commands for category support
- [ ] Fix existing commands to work with new structure
- [ ] Generate updated TypeScript bindings
- [ ] Test backend functionality

### Week 2: Core UI Components
- [ ] Update ListDisplay component
- [ ] Create CategorySection component
- [ ] Update AddItemForm with category support
- [ ] Basic category display working

### Week 3: Category Management
- [ ] Implement CategoryManager component
- [ ] Add context menus and actions
- [ ] Drag & drop functionality
- [ ] Category CRUD operations

### Week 4: Polish & Mobile
- [ ] Mobile-specific UI adaptations
- [ ] Keyboard shortcuts (desktop)
- [ ] Visual enhancements
- [ ] Testing and bug fixes

## Technical Considerations

### 1. State Management
- **Desktop**: Update Zustand stores to handle categories
- **Mobile**: Ensure SQLite integration works with categories
- **Sync**: Verify category data syncs properly between devices

### 2. Performance
- **Large lists**: Virtualization for categories with many items
- **Memory**: Efficient rendering of collapsed categories
- **Search**: Indexed search across categories

### 3. Data Migration
- **Backward compatibility**: Handle lists without categories
- **Upgrade path**: Migrate existing lists gracefully
- **Sync conflicts**: Handle category conflicts in CRDT sync

### 4. Error Handling
- **Invalid categories**: Handle malformed category names
- **Sync errors**: Graceful degradation when sync fails
- **UI errors**: User-friendly error messages

## Testing Strategy

### Unit Tests
- [ ] Category data structure serialization/deserialization
- [ ] Category parsing logic
- [ ] Item movement between categories

### Integration Tests
- [ ] Tauri command functionality
- [ ] Frontend-backend communication
- [ ] Category persistence

### E2E Tests
- [ ] Create category workflow
- [ ] Move items between categories
- [ ] Delete category with items
- [ ] Category display and interaction

### Manual Testing
- [ ] Mobile touch interactions
- [ ] Desktop keyboard shortcuts
- [ ] Cross-platform sync
- [ ] Performance with large lists

## Success Criteria

### Functional Requirements
- ✅ Users can create, rename, and delete categories
- ✅ Users can move items between categories
- ✅ Categories display properly in both apps
- ✅ Backward compatibility with non-categorized lists
- ✅ Category data syncs between devices

### UX Requirements
- ✅ Intuitive category management
- ✅ Fast item categorization
- ✅ Clear visual hierarchy
- ✅ Responsive mobile interface
- ✅ Efficient keyboard navigation (desktop)

### Technical Requirements
- ✅ No performance degradation
- ✅ Proper error handling
- ✅ Data integrity maintained
- ✅ TypeScript type safety
- ✅ Cross-platform compatibility

## Future Enhancements

### Advanced Features (Post-MVP)
1. **Nested categories** (subcategories)
2. **Category templates** for new lists
3. **Smart categorization** (AI-powered suggestions)
4. **Category sharing** between lists
5. **Category-based notifications**
6. **Advanced filtering** and search
7. **Category analytics** and insights

### Integration Opportunities
1. **Calendar integration** (date-based categories)
2. **Location-based** categories
3. **Tag system** integration
4. **External service** sync (Todoist, etc.)

---

## Notes for Developers

### Key Files to Modify
```
apps/lst-desktop/src-tauri/src/lib.rs          # Tauri commands
apps/lst-mobile/src-tauri/src/lib.rs           # Tauri commands
apps/lst-desktop/src/components/ListDisplay.tsx # Main list view
apps/lst-mobile/src/components/ListDisplay.tsx  # Main list view
apps/lst-desktop/src/components/AddItemForm.tsx # Add item form
apps/lst-mobile/src/components/AddItemForm.tsx  # Add item form
```

### TypeScript Types to Add
```typescript
interface Category {
  name: string;
  items: ListItem[];
}

interface List {
  metadata: ListMetadata;
  uncategorized_items: ListItem[];
  categories: Category[];
}
```

### Tauri Commands to Implement
```rust
create_category(list_name: String, category_name: String)
move_item_to_category(list_name: String, item_anchor: String, category_name: Option<String>)
delete_category(list_name: String, category_name: String)
get_categories(list_name: String)
rename_category(list_name: String, old_name: String, new_name: String)
```

This implementation plan provides a comprehensive roadmap for adding category support to both desktop and mobile applications while maintaining backward compatibility and ensuring a great user experience across all platforms.