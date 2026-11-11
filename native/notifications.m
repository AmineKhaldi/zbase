// macOS banners for incoming chat messages. Clicking a banner hands the
// conversation and message ids back to Rust through `on_click`.
#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>
#import <UserNotifications/UserNotifications.h>
#import <stdatomic.h>

typedef void (*ZBaseNotificationClick)(const char *conversation_id, const char *message_id);

@interface ZBaseNotificationDelegate : NSObject <UNUserNotificationCenterDelegate>
@property ZBaseNotificationClick onClick;
@end

@implementation ZBaseNotificationDelegate
- (void)userNotificationCenter:(UNUserNotificationCenter *)center
       willPresentNotification:(UNNotification *)notification
         withCompletionHandler:(void (^)(UNNotificationPresentationOptions))completionHandler {
    (void)center; (void)notification;
    // Rust already skipped the conversation the user is reading, so the
    // banner shows even while zBase is frontmost, as in Keybase.
    completionHandler(UNNotificationPresentationOptionBanner | UNNotificationPresentationOptionList |
                      UNNotificationPresentationOptionSound);
}
- (void)userNotificationCenter:(UNUserNotificationCenter *)center
    didReceiveNotificationResponse:(UNNotificationResponse *)response
             withCompletionHandler:(void (^)(void))completionHandler {
    (void)center;
    NSDictionary *info = response.notification.request.content.userInfo;
    NSString *conversation = info[@"conversation_id"];
    NSString *message = info[@"message_id"];
    if ([response.actionIdentifier isEqualToString:UNNotificationDefaultActionIdentifier] &&
        [conversation isKindOfClass:NSString.class] && [message isKindOfClass:NSString.class] &&
        self.onClick) {
        self.onClick(conversation.UTF8String, message.UTF8String);
    }
    completionHandler();
}
@end

static ZBaseNotificationDelegate *delegate;

// What macOS currently lets zBase show; mirrors NotificationPermission in Rust.
enum { PermissionUnavailable, PermissionNotAsked, PermissionDenied, PermissionAllowed, PermissionSilenced };
static _Atomic int permission = PermissionUnavailable;

static void refresh_permission(void) {
    [UNUserNotificationCenter.currentNotificationCenter
        getNotificationSettingsWithCompletionHandler:^(UNNotificationSettings *settings) {
            int value;
            switch (settings.authorizationStatus) {
                case UNAuthorizationStatusNotDetermined: value = PermissionNotAsked; break;
                case UNAuthorizationStatusDenied: value = PermissionDenied; break;
                default: value = settings.alertStyle == UNAlertStyleNone ? PermissionSilenced : PermissionAllowed;
            }
            atomic_store(&permission, value);
        }];
}

static void request_permission(void) {
    [UNUserNotificationCenter.currentNotificationCenter
        requestAuthorizationWithOptions:UNAuthorizationOptionAlert | UNAuthorizationOptionSound |
                                        UNAuthorizationOptionBadge
                      completionHandler:^(BOOL granted, NSError *error) {
                          if (!granted) NSLog(@"zBase notifications not authorized: %@", error);
                          refresh_permission();
                      }];
}

// UNUserNotificationCenter raises when the process has no app bundle, as
// with `cargo run`; those builds post nothing.
bool zbase_notifications_start(ZBaseNotificationClick on_click) {
    if (delegate) return true;
    NSBundle *bundle = NSBundle.mainBundle;
    if (!bundle.bundleIdentifier || ![bundle.bundlePath.pathExtension isEqualToString:@"app"]) return false;
    delegate = [[ZBaseNotificationDelegate alloc] init];
    delegate.onClick = on_click;
    UNUserNotificationCenter *center = UNUserNotificationCenter.currentNotificationCenter;
    center.delegate = delegate;
    request_permission();
    return true;
}

// Asks macOS for a fresh answer; read it later with zbase_notifications_permission.
void zbase_notifications_refresh_permission(void) {
    if (delegate) refresh_permission();
}

int zbase_notifications_permission(void) { return atomic_load(&permission); }

// Shows the macOS prompt when zBase has not asked yet.
void zbase_notifications_request_permission(void) {
    if (delegate) request_permission();
}

void zbase_notifications_open_settings(void) {
    NSString *bundle = NSBundle.mainBundle.bundleIdentifier;
    if (!delegate || !bundle) return;
    NSString *url = [@"x-apple.systempreferences:com.apple.Notifications-Settings.extension?id="
        stringByAppendingString:bundle];
    [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:url]];
}

// A copy of the sender's avatar to show beside the text. macOS moves an
// attachment's file into its own store, so the cached original is never handed over.
static UNNotificationAttachment *avatar_attachment(NSString *path) {
    if (path.length == 0) return nil;
    NSString *name = [NSUUID.UUID.UUIDString stringByAppendingPathExtension:path.pathExtension];
    NSURL *copy = [NSURL fileURLWithPath:[NSTemporaryDirectory() stringByAppendingPathComponent:name]];
    if (![NSFileManager.defaultManager copyItemAtURL:[NSURL fileURLWithPath:path] toURL:copy error:nil]) return nil;
    NSError *error = nil;
    UNNotificationAttachment *attachment = [UNNotificationAttachment attachmentWithIdentifier:@"avatar"
                                                                                         URL:copy
                                                                                     options:nil
                                                                                       error:&error];
    if (!attachment) {
        NSLog(@"zBase notification avatar skipped: %@", error);
        [NSFileManager.defaultManager removeItemAtURL:copy error:nil];
    }
    return attachment;
}

void zbase_notifications_post(const char *title, const char *body, const char *conversation_id,
                              const char *message_id, const char *image_path, bool sound) {
    if (!delegate) return;
    UNMutableNotificationContent *content = [[UNMutableNotificationContent alloc] init];
    content.title = [NSString stringWithUTF8String:title] ?: @"";
    content.body = [NSString stringWithUTF8String:body] ?: @"";
    NSString *conversation = [NSString stringWithUTF8String:conversation_id] ?: @"";
    NSString *message = [NSString stringWithUTF8String:message_id] ?: @"";
    content.threadIdentifier = conversation;
    content.userInfo = @{@"conversation_id": conversation, @"message_id": message};
    if (sound) content.sound = UNNotificationSound.defaultSound;
    UNNotificationAttachment *avatar = avatar_attachment([NSString stringWithUTF8String:image_path]);
    if (avatar) content.attachments = @[avatar];
    NSString *identifier = [NSString stringWithFormat:@"%@/%@", conversation, message];
    UNNotificationRequest *request = [UNNotificationRequest requestWithIdentifier:identifier
                                                                          content:content
                                                                          trigger:nil];
    [UNUserNotificationCenter.currentNotificationCenter
        addNotificationRequest:request
         withCompletionHandler:^(NSError *error) {
             if (error) NSLog(@"zBase notification failed: %@", error);
         }];
}

double zbase_seconds_since_user_input(void) {
    return CGEventSourceSecondsSinceLastEventType(kCGEventSourceStateCombinedSessionState, kCGAnyInputEventType);
}
