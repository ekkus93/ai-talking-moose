struct PowerCallbackContext {
    sender: Sender<PowerEvent>,
    root_port: AtomicU32,
}

unsafe extern "C" fn power_callback(
    refcon: *mut c_void,
    _service: IoObject,
    message_type: u32,
    message_argument: *mut c_void,
) {
    if refcon.is_null() {
        return;
    }
    // SAFETY: refcon points to the boxed PowerCallbackContext retained by the observer thread.
    let context = unsafe { &*(refcon.cast::<PowerCallbackContext>()) };
    match message_type {
        IO_MESSAGE_CAN_SYSTEM_SLEEP => {
            let root_port = context.root_port.load(Ordering::SeqCst);
            if root_port != 0 {
                // SAFETY: message_argument is the power-notification token documented by IOKit.
                unsafe { io_allow_power_change(root_port, message_argument as isize) };
            }
        }
        IO_MESSAGE_SYSTEM_WILL_SLEEP => {
            let _ = context.sender.try_send(PowerEvent::Sleep);
            let root_port = context.root_port.load(Ordering::SeqCst);
            if root_port != 0 {
                // SAFETY: the system sleep notification must be acknowledged with its token.
                unsafe { io_allow_power_change(root_port, message_argument as isize) };
            }
        }
        IO_MESSAGE_SYSTEM_HAS_POWERED_ON => {
            let _ = context.sender.try_send(PowerEvent::Wake);
        }
        _ => {}
    }
}

fn cleanup_power_registration(
    root_port: IoConnect,
    notification_port: IONotificationPortRef,
    notifier: &mut IoObject,
) {
    // SAFETY: these handles were returned by io_register_for_system_power and are released once.
    unsafe {
        if *notifier != 0 {
            let _ = io_deregister_for_system_power(notifier);
        }
        if !notification_port.is_null() {
            io_notification_port_destroy(notification_port);
        }
        if root_port != 0 {
            let _ = io_service_close(root_port);
        }
    }
}

pub(super) fn start_power_events(
    sender: Sender<PowerEvent>,
) -> ObserverResult<SystemPowerObserver> {
    let running = Arc::new(AtomicBool::new(true));
    let thread_running = running.clone();
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    let observer_thread = thread::spawn(move || {
        let context = Box::new(PowerCallbackContext {
            sender,
            root_port: AtomicU32::new(0),
        });
        let context_ptr = Box::into_raw(context);
        let mut notification_port: IONotificationPortRef = ptr::null_mut();
        let mut notifier = 0_u32;
        // SAFETY: context_ptr remains allocated for the complete registration lifetime.
        let root_port = unsafe {
            io_register_for_system_power(
                context_ptr.cast::<c_void>(),
                &mut notification_port,
                power_callback,
                &mut notifier,
            )
        };
        if root_port == 0 || notification_port.is_null() {
            cleanup_power_registration(root_port, notification_port, &mut notifier);
            // SAFETY: registration failed, so no run-loop callback retains context_ptr.
            unsafe { drop(Box::from_raw(context_ptr)) };
            let _ = ready_tx.send(Err(ObserverErrorCode::RegistrationFailed));
            return;
        }
        // SAFETY: context_ptr remains valid until after run-loop processing stops.
        unsafe { (*context_ptr).root_port.store(root_port, Ordering::SeqCst) };
        // SAFETY: notification_port is valid after successful registration.
        let source = unsafe { io_notification_port_get_run_loop_source(notification_port) };
        if source.is_null() {
            cleanup_power_registration(root_port, notification_port, &mut notifier);
            // SAFETY: no run-loop source was installed, so callbacks cannot race cleanup.
            unsafe { drop(Box::from_raw(context_ptr)) };
            let _ = ready_tx.send(Err(ObserverErrorCode::RegistrationFailed));
            return;
        }
        // SAFETY: this dedicated thread owns its current run loop for the observer lifetime.
        let run_loop = unsafe { cf_run_loop_get_current() };
        // SAFETY: source and default mode are valid CoreFoundation objects.
        unsafe { cf_run_loop_add_source(run_loop, source, cf_run_loop_default_mode) };
        let _ = ready_tx.send(Ok(()));

        while thread_running.load(Ordering::SeqCst) {
            // SAFETY: run the dedicated observer run loop in bounded slices so stop can join.
            unsafe { cf_run_loop_run_in_mode(cf_run_loop_default_mode, 0.25, 1) };
            thread::sleep(Duration::from_millis(10));
        }

        cleanup_power_registration(root_port, notification_port, &mut notifier);
        // SAFETY: the run loop has stopped and registration is torn down before freeing context.
        unsafe { drop(Box::from_raw(context_ptr)) };
    });

    match ready_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(())) => ObserverResult::Available(SystemPowerObserver {
            running,
            thread: Some(observer_thread),
        }),
        Ok(Err(code)) => {
            let _ = observer_thread.join();
            ObserverResult::Error(code)
        }
        Err(_) => {
            running.store(false, Ordering::SeqCst);
            drop(observer_thread);
            ObserverResult::Error(ObserverErrorCode::RegistrationFailed)
        }
    }
}
