#import <AVFoundation/AVFoundation.h>
#include <stdint.h>
#include <dispatch/dispatch.h>

// AVAuthorizationStatus: 0 notDetermined, 1 restricted, 2 denied, 3 authorized.

int32_t mabel_mic_authorization_status(void) {
    return (int32_t)[AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
}

// Blocking. Must run off the main thread so the TCC dialog can use the run loop.
int32_t mabel_mic_request_access(void) {
    AVAuthorizationStatus status =
        [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
    if (status == AVAuthorizationStatusAuthorized) {
        return 1;
    }
    if (status == AVAuthorizationStatusDenied || status == AVAuthorizationStatusRestricted) {
        return 0;
    }

    dispatch_semaphore_t sem = dispatch_semaphore_create(0);
    __block int32_t granted = 0;
    [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio
                             completionHandler:^(BOOL ok) {
                               granted = ok ? 1 : 0;
                               dispatch_semaphore_signal(sem);
                             }];
    dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, (int64_t)120 * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(sem, timeout) != 0) {
        return 0;
    }
    return granted;
}
