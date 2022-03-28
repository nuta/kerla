#ifndef __LIST_H__
#define __LIST_H__

#include <printf.h>
#include <types.h>

//  Usage:
//
//    struct element {
//        struct list_head next;
//        int foo;
//    };
//
//    LIST_FOR_EACH(elem, &elems, struct element, next) {
//        printf("foo: %d", elem->foo);
//    }
//
#define LIST_CONTAINER(head, container, field)                                 \
    ((container *) ((vaddr_t) (head) -offsetof(container, field)))
#define LIST_FOR_EACH(elem, list, container, field)                            \
    for (container *elem = LIST_CONTAINER((list)->next, container, field),     \
                   *__next = NULL;                                             \
         (&elem->field != (list)                                               \
          && (__next = LIST_CONTAINER(elem->field.next, container, field)));   \
         elem = __next)
#define LIST_FOR_EACH_REV(elem, list, container, field)                        \
    for (container *elem = LIST_CONTAINER((list)->prev, container, field),     \
                   *__prev = NULL;                                             \
         (&elem->field != (list)                                               \
          && (__prev = LIST_CONTAINER(elem->field.prev, container, field)));   \
         elem = __prev)

struct list_head {
    struct list_head *prev;
    struct list_head *next;
};

typedef struct list_head list_t;
typedef struct list_head list_elem_t;

static inline bool list_is_empty(list_t *list) {
    return list->next == list;
}

static inline bool list_is_null_elem(list_elem_t *elem) {
    return elem->prev == NULL || elem->next == NULL;
}

static inline size_t list_len(list_t *list) {
    size_t len = 0;
    struct list_head *node = list->next;
    while (node != list) {
        len++;
        node = node->next;
    }

    return len;
}

static inline bool list_contains(list_t *list, list_elem_t *elem) {
    list_elem_t *node = list->next;
    while (node != list) {
        if (node == elem) {
            return true;
        }
        node = node->next;
    }

    return false;
}

// Inserts a new element between `prev` and `next`.
static inline void list_insert(list_elem_t *prev, list_elem_t *next,
                               list_elem_t *new) {
    new->prev = prev;
    new->next = next;
    next->prev = new;
    prev->next = new;
}

// Initializes a list.
static inline void list_init(list_t *list) {
    list->prev = list;
    list->next = list;
}

// Invalidates a list element.
static inline void list_nullify(list_elem_t *elem) {
    elem->prev = NULL;
    elem->next = NULL;
}

// Removes a element from the list.
static inline void list_remove(list_elem_t *elem) {
    if (!elem->next) {
        // The element is not in a list.
        return;
    }

    elem->prev->next = elem->next;
    elem->next->prev = elem->prev;

    // Invalidate the element as they're no longer in the list.
    list_nullify(elem);
}

// Prepends a element into the list.
static inline void list_push_front(list_t *list, list_elem_t *new_head) {
    DEBUG_ASSERT(!list_contains(list, new_head));
    list_insert(list->next, list, new_head);
}

// Appends a element into the list.
static inline void list_push_back(list_t *list, list_elem_t *new_tail) {
    DEBUG_ASSERT(!list_contains(list, new_tail));
    list_insert(list->prev, list, new_tail);
}

// Get and removes the first element from the list.
static inline list_t *list_pop_front(list_t *list) {
    struct list_head *head = list->next;
    if (head == list) {
        return NULL;
    }

    // list <-> head <-> next => list <-> next
    struct list_head *next = head->next;
    list->next = next;
    next->prev = list;

    // Invalidate the element as they're no longer in the list.
    list_nullify(head);
    return head;
}

#define LIST_POP_FRONT(list, container, field)                                 \
    ({                                                                         \
        list_elem_t *__elem = list_pop_front(list);                            \
        (__elem) ? LIST_CONTAINER(__elem, container, field) : NULL;            \
    })

#endif
