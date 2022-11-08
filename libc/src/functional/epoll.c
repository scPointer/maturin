/// Reference: https://github.com/torvalds/linux/blob/master/tools/testing/selftests/filesystems/epoll/epoll_wakeup_test.c

#include <errno.h>
#include <poll.h>
#include <pthread.h>
#include <string.h>
#include <stdio.h>
#include <unistd.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <sys/eventfd.h>
#include "test.h"

#define TEST(c, ...) ((c) ? 1 : (t_error(#c" failed: " __VA_ARGS__),0))
#define TESTE(c) (errno=0, TEST(c, "errno = %s\n", strerror(errno)))

struct epoll_mtcontext
{
    int efd[3];
    int pfd[4];
    volatile int count;

    pthread_t main;
    pthread_t waiter;
};

static void *waiter_entry1a(void *data)
{
    struct epoll_event e;
    struct epoll_mtcontext *ctx = data;

    if (epoll_wait(ctx->efd[0], &e, 1, -1) > 0)
        __sync_fetch_and_add(&ctx->count, 1);

    return NULL;
}

static void *waiter_entry1ap(void *data)
{
    struct pollfd pfd;
    struct epoll_event e;
    struct epoll_mtcontext *ctx = data;

    pfd.fd = ctx->efd[0];
    pfd.events = POLLIN;
    if (poll(&pfd, 1, -1) > 0) {
        if (epoll_wait(ctx->efd[0], &e, 1, 0) > 0)
            __sync_fetch_and_add(&ctx->count, 1);
    }

    return NULL;
}

static void *emitter_entry1(void *data)
{
    struct epoll_mtcontext *ctx = data;

    sleep(1);
    write(ctx->pfd[1], "w", 1);

    return NULL;
}

/*
 *          t0
 *           | (ew)
 *          e0
 *           | (lt)
 *          p0
 */
void epoll1(void)
{
    int efd;
    int pfd[2];
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (ew)
 *          e0
 *           | (et)
 *          p0
 */
void epoll2(void)
{
    int efd;
    int pfd[2];
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
}


/*
 *           t0
 *            | (ew)
 *           e0
 *     (lt) /  \ (lt)
 *        p0    p2
 */
void epoll3(void)
{
    int efd;
    int pfd[4];
    struct epoll_event events[2];

    TESTE(pipe(&pfd[0]) == 0);
    TESTE(pipe(&pfd[2]) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    events[0].events = EPOLLIN;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], events) == 0);

    events[0].events = EPOLLIN;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[2], events) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);
    TESTE(write(pfd[3], "w", 1) == 1);

    TEST(epoll_wait(efd, events, 2, 0) == 2, "epoll_wait return value should be 2\n");
    TEST(epoll_wait(efd, events, 2, 0) == 2, "epoll_wait return value should be 2\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
    close(pfd[2]);
    close(pfd[3]);
}


/*
 *           t0
 *            | (ew)
 *           e0
 *     (et) /  \ (et)
 *        p0    p2
 */
void epoll4(void)
{
    int efd;
    int pfd[4];
    struct epoll_event events[2];

    TESTE(pipe(&pfd[0]) == 0);
    TESTE(pipe(&pfd[2]) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    events[0].events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], events) == 0);

    events[0].events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[2], events) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);
    TESTE(write(pfd[3], "w", 1) == 1);

    TEST(epoll_wait(efd, events, 2, 0) == 2, "epoll_wait return value should be 2\n");
    TEST(epoll_wait(efd, events, 2, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
    close(pfd[2]);
    close(pfd[3]);
}

/*
 *          t0
 *           | (p)
 *          e0
 *           | (lt)
 *          p0
 */
void epoll5(void)
{
    int efd;
    int pfd[2];
    struct pollfd poll_fd;
    struct epoll_event e;

    TESTE(pipe(&pfd[0]) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (p)
 *          e0
 *           | (et)
 *          p0
 */
void epoll6(void)
{
    int efd;
    int pfd[2];
    struct pollfd poll_fd;
    struct epoll_event e;

    TESTE(pipe(&pfd[0]) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 0, "poll return value should be 0\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *           t0
 *            | (p)
 *           e0
 *     (lt) /  \ (lt)
 *        p0    p2
 */
void epoll7(void)
{
    int efd;
    int pfd[4];
    struct pollfd poll_fd;
    struct epoll_event events[2];

    TESTE(pipe(&pfd[0]) == 0);
    TESTE(pipe(&pfd[2]) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    events[0].events = EPOLLIN;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], events) == 0);

    events[0].events = EPOLLIN;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[2], events) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);
    TESTE(write(pfd[3], "w", 1) == 1);

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd, events, 2, 0) == 2, "epoll_wait return value should be 2\n");

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd, events, 2, 0) == 2, "epoll_wait return value should be 2\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
    close(pfd[2]);
    close(pfd[3]);
}

/*
 *           t0
 *            | (p)
 *           e0
 *     (et) /  \ (et)
 *        p0    p2
 */
void epoll8(void)
{
    int efd;
    int pfd[4];
    struct pollfd poll_fd;
    struct epoll_event events[2];

    TESTE(pipe(&pfd[0]) == 0);
    TESTE(pipe(&pfd[2]) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    events[0].events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], events) == 0);

    events[0].events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[2], events) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);
    TESTE(write(pfd[3], "w", 1) == 1);

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd, events, 2, 0) == 2, "epoll_wait return value should be 2\n");

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 0, "poll return value should be 0\n");
    TEST(epoll_wait(efd, events, 2, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
    close(pfd[2]);
    close(pfd[3]);
}

/*
 *        t0    t1
 *     (ew) \  / (ew)
 *           e0
 *            | (lt)
 *           s0
 */
void epoll9(void)
{
    pthread_t emitter;
    struct epoll_event e;
    struct epoll_mtcontext ctx = { 0 };

    TESTE(pipe(ctx.pfd) == 0);

    ctx.efd[0] = epoll_create(1);
    TESTE(ctx.efd[0] >= 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(ctx.efd[0], EPOLL_CTL_ADD, ctx.pfd[0], &e) == 0);

    ctx.main = pthread_self();
    TESTE(pthread_create(&ctx.waiter, NULL, waiter_entry1a, &ctx) == 0);
    TESTE(pthread_create(&emitter, NULL, emitter_entry1, &ctx) == 0);

    if (epoll_wait(ctx.efd[0], &e, 1, -1) > 0)
        __sync_fetch_and_add(&ctx.count, 1);

    TESTE(pthread_join(ctx.waiter, NULL) == 0);
    TEST(ctx.count == 2, "counter value should be 2\n");

    TESTE(pthread_join(emitter, NULL) == 0);

    close(ctx.efd[0]);
    close(ctx.pfd[0]);
    close(ctx.pfd[1]);
}

/*
 *        t0    t1
 *     (ew) \  / (p)
 *           e0
 *            | (lt)
 *           s0
 */
void epoll13(void)
{
    pthread_t emitter;
    struct epoll_event e;
    struct epoll_mtcontext ctx = { 0 };

    TESTE(pipe(ctx.pfd) == 0);

    ctx.efd[0] = epoll_create(1);
    TESTE(ctx.efd[0] >= 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(ctx.efd[0], EPOLL_CTL_ADD, ctx.pfd[0], &e) == 0);

    ctx.main = pthread_self();
    TESTE(pthread_create(&ctx.waiter, NULL, waiter_entry1ap, &ctx) == 0);
    TESTE(pthread_create(&emitter, NULL, emitter_entry1, &ctx) == 0);

    if (epoll_wait(ctx.efd[0], &e, 1, -1) > 0)
        __sync_fetch_and_add(&ctx.count, 1);

    TESTE(pthread_join(ctx.waiter, NULL) == 0);
    TEST(ctx.count == 2, "counter value should be 2\n");

    TESTE(pthread_join(emitter, NULL) == 0);

    close(ctx.efd[0]);
    close(ctx.pfd[0]);
    close(ctx.pfd[1]);
}

/*
 *          t0
 *           | (ew)
 *          e0
 *           | (lt)
 *          e1
 *           | (lt)
 *          p0
 */
void epoll17(void)
{
    int efd[2];
    int pfd[2];
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd[0] = epoll_create(1);
    TESTE(efd[0] >= 0);

    efd[1] = epoll_create(1);
    TESTE(efd[1] >= 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd[1], EPOLL_CTL_ADD, pfd[0], &e) == 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd[0], EPOLL_CTL_ADD, efd[1], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    close(efd[0]);
    close(efd[1]);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (ew)
 *          e0
 *           | (lt)
 *          e1
 *           | (et)
 *          p0
 */
void epoll18(void)
{
    int efd[2];
    int pfd[2];
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd[0] = epoll_create(1);
    TESTE(efd[0] >= 0);

    efd[1] = epoll_create(1);
    TESTE(efd[1] >= 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd[1], EPOLL_CTL_ADD, pfd[0], &e) == 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd[0], EPOLL_CTL_ADD, efd[1], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    close(efd[0]);
    close(efd[1]);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (ew)
 *          e0
 *           | (et)
 *          e1
 *           | (lt)
 *          p0
 */
void epoll19(void)
{
    int efd[2];
    int pfd[2];
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd[0] = epoll_create(1);
    TESTE(efd[0] >= 0);

    efd[1] = epoll_create(1);
    TESTE(efd[1] >= 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd[1], EPOLL_CTL_ADD, pfd[0], &e) == 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd[0], EPOLL_CTL_ADD, efd[1], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd[0]);
    close(efd[1]);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (ew)
 *          e0
 *           | (et)
 *          e1
 *           | (et)
 *          p0
 */
void epoll20(void)
{
    int efd[2];
    int pfd[2];
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd[0] = epoll_create(1);
    TESTE(efd[0] >= 0);

    efd[1] = epoll_create(1);
    TESTE(efd[1] >= 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd[1], EPOLL_CTL_ADD, pfd[0], &e) == 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd[0], EPOLL_CTL_ADD, efd[1], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd[0]);
    close(efd[1]);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (p)
 *          e0
 *           | (lt)
 *          e1
 *           | (lt)
 *          s0
 */
void epoll21(void)
{
    int efd[2];
    int pfd[2];
    struct pollfd poll_fd;
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd[0] = epoll_create(1);
    TESTE(efd[0] >= 0);

    efd[1] = epoll_create(1);
    TESTE(efd[1] >= 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd[1], EPOLL_CTL_ADD, pfd[0], &e) == 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd[0], EPOLL_CTL_ADD, efd[1], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    poll_fd.fd = efd[0];
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    poll_fd.fd = efd[0];
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    close(efd[0]);
    close(efd[1]);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (p)
 *          e0
 *           | (lt)
 *          e1
 *           | (et)
 *          s0
 */
void epoll22(void)
{
    int efd[2];
    int pfd[2];
    struct pollfd poll_fd;
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd[0] = epoll_create(1);
    TESTE(efd[0] >= 0);

    efd[1] = epoll_create(1);
    TESTE(efd[1] >= 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd[1], EPOLL_CTL_ADD, pfd[0], &e) == 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd[0], EPOLL_CTL_ADD, efd[1], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    poll_fd.fd = efd[0];
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    poll_fd.fd = efd[0];
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    close(efd[0]);
    close(efd[1]);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (p)
 *          e0
 *           | (et)
 *          e1
 *           | (lt)
 *          s0
 */
void epoll23(void)
{
    int efd[2];
    int pfd[2];
    struct pollfd poll_fd;
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd[0] = epoll_create(1);
    TESTE(efd[0] >= 0);

    efd[1] = epoll_create(1);
    TESTE(efd[1] >= 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd[1], EPOLL_CTL_ADD, pfd[0], &e) == 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd[0], EPOLL_CTL_ADD, efd[1], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    poll_fd.fd = efd[0];
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    poll_fd.fd = efd[0];
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 0, "poll return value should be 0\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd[0]);
    close(efd[1]);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (p)
 *          e0
 *           | (et)
 *          e1
 *           | (et)
 *          s0
 */
void epoll24(void)
{
    int efd[2];
    int pfd[2];
    struct pollfd poll_fd;
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd[0] = epoll_create(1);
    TESTE(efd[0] >= 0);

    efd[1] = epoll_create(1);
    TESTE(efd[1] >= 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd[1], EPOLL_CTL_ADD, pfd[0], &e) == 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd[0], EPOLL_CTL_ADD, efd[1], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    poll_fd.fd = efd[0];
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    poll_fd.fd = efd[0];
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 0, "poll return value should be 0\n");
    TEST(epoll_wait(efd[0], &e, 1, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd[0]);
    close(efd[1]);
    close(pfd[0]);
    close(pfd[1]);
}

int main(void)
{
    epoll1();
    // epoll2();
    epoll3();
    // epoll4();
    epoll5();
    // epoll6();
    epoll7();
    // epoll8();
    epoll9();
    epoll13();
    epoll17();
    // epoll18();
    // epoll19();
    // epoll20();
    epoll21();
    // epoll22();
    // epoll23();
    // epoll24();
}